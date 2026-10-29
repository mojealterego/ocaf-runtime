#!/usr/bin/env python3
"""OCAF stage 334 bounded reference memory algorithms.

LosslessTenantArchive is a real byte-exact compressed store, not a claim that
NICE/R3Mem is implemented. SurpriseGatedMemory is a deterministic bounded
surprise-gated vector memory, not a trained Titans neural architecture.
"""
from __future__ import annotations

from collections import OrderedDict
import hashlib
import json
import math
import os
from pathlib import Path
import re
import secrets
import stat
import struct
import zlib

MAGIC = b"OCAF334L1\x00"
MAX_BLOB = 4 * 1024 * 1024
_ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,63}\Z")
_DIGEST = re.compile(r"[0-9a-f]{64}\Z")


class MemoryContractError(ValueError):
    pass


def _valid_id(value: object) -> bool:
    return isinstance(value, str) and bool(_ID.fullmatch(value)) and value not in (".", "..")


def _canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()


class LosslessTenantArchive:
    """Tenant-private content-addressed zlib objects with full SHA-256 checking."""

    def __init__(self, root: Path, tenant: str):
        root = Path(root)
        if not root.is_absolute() or not _valid_id(tenant):
            raise MemoryContractError("invalid archive root or tenant")
        try:
            if root.is_symlink() or not root.is_dir():
                raise MemoryContractError("archive root must be a real directory")
            meta = root.stat()
            if meta.st_uid != os.geteuid() or stat.S_IMODE(meta.st_mode) & 0o077:
                raise MemoryContractError("archive root must be private and owned")
            target = root / tenant
            target.mkdir(mode=0o700, exist_ok=True)
            if target.is_symlink() or not target.is_dir():
                raise MemoryContractError("invalid tenant directory")
            meta = target.stat()
            if meta.st_uid != os.geteuid() or stat.S_IMODE(meta.st_mode) & 0o077:
                raise MemoryContractError("tenant directory is not private")
        except OSError as error:
            raise MemoryContractError("archive initialization failed") from error
        self.directory = target

    def put(self, data: bytes) -> str:
        if not isinstance(data, bytes) or len(data) > MAX_BLOB:
            raise MemoryContractError("invalid or oversized payload")
        digest = hashlib.sha256(data).hexdigest()
        target = self.directory / (digest + ".zlib")
        if target.exists() or target.is_symlink():
            if self.get(digest) != data:
                raise MemoryContractError("existing object differs")
            return digest
        record = MAGIC + struct.pack(">Q", len(data)) + zlib.compress(data, level=9)
        temporary = self.directory / (".tmp-" + secrets.token_hex(16))
        flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
        try:
            fd = os.open(temporary, flags, 0o600)
            with os.fdopen(fd, "wb") as file:
                file.write(record)
                file.flush()
                os.fsync(file.fileno())
            try:
                # Atomic no-clobber install: another writer cannot replace
                # an existing immutable digest object with different bytes.
                os.link(temporary, target, follow_symlinks=False)
            except FileExistsError:
                if self.get(digest) != data:
                    raise MemoryContractError("concurrent object integrity failure")
            directory_fd = os.open(self.directory, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
            try:
                os.fsync(directory_fd)
            finally:
                os.close(directory_fd)
        except OSError as error:
            raise MemoryContractError("archive write failed") from error
        finally:
            try:
                temporary.unlink(missing_ok=True)
            except OSError:
                pass
        return digest

    def get(self, digest: str) -> bytes:
        if not isinstance(digest, str) or not _DIGEST.fullmatch(digest):
            raise MemoryContractError("invalid digest")
        target = self.directory / (digest + ".zlib")
        try:
            fd = os.open(target, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
            with os.fdopen(fd, "rb") as file:
                meta = os.fstat(file.fileno())
                if (not stat.S_ISREG(meta.st_mode)
                        or meta.st_uid != os.geteuid()
                        or stat.S_IMODE(meta.st_mode) & 0o077
                        or meta.st_size > MAX_BLOB + 65536):
                    raise MemoryContractError("unsafe archive object")
                record = file.read(MAX_BLOB + 65537)
        except OSError as error:
            raise MemoryContractError("archive object unavailable") from error
        if len(record) < len(MAGIC) + 8 or not record.startswith(MAGIC):
            raise MemoryContractError("invalid archive header")
        original_size = struct.unpack(">Q", record[len(MAGIC):len(MAGIC) + 8])[0]
        if original_size > MAX_BLOB:
            raise MemoryContractError("declared object exceeds limit")
        decoder = zlib.decompressobj()
        try:
            output = decoder.decompress(record[len(MAGIC) + 8:], MAX_BLOB + 1)
        except zlib.error as error:
            raise MemoryContractError("corrupt compressed object") from error
        if (len(output) != original_size or not decoder.eof
                or decoder.unconsumed_tail or decoder.unused_data
                or hashlib.sha256(output).hexdigest() != digest):
            raise MemoryContractError("lossless archive integrity mismatch")
        return output


class SurpriseGatedMemory:
    """Bounded vector EMA with squared-L2 surprise gate and deterministic LRU."""

    def __init__(self, dimension: int, capacity: int, threshold: float, decay: float):
        if (type(dimension) is not int or not 1 <= dimension <= 4096
                or type(capacity) is not int or not 1 <= capacity <= 10_000
                or not isinstance(threshold, (int, float))
                or not math.isfinite(threshold) or threshold < 0
                or not isinstance(decay, (int, float))
                or not math.isfinite(decay) or not 0 < decay <= 1):
            raise MemoryContractError("invalid memory configuration")
        self.dimension = dimension
        self.capacity = capacity
        self.threshold = float(threshold)
        self.decay = float(decay)
        self._entries: OrderedDict[tuple[str, str], list[float]] = OrderedDict()

    @property
    def size(self) -> int:
        return len(self._entries)

    def _vector(self, vector: object) -> list[float]:
        if (not isinstance(vector, (list, tuple))
                or len(vector) != self.dimension
                or any(type(x) not in (int, float) or not math.isfinite(x) for x in vector)):
            raise MemoryContractError("invalid vector")
        return [float(x) for x in vector]

    def observe(self, tenant: str, key: str, vector: object) -> bool:
        if not _valid_id(tenant) or not _valid_id(key):
            raise MemoryContractError("invalid memory identity")
        values = self._vector(vector)
        identity = (tenant, key)
        previous = self._entries.get(identity)
        if previous is not None:
            surprise = sum((a - b) ** 2 for a, b in zip(previous, values))
            if not math.isfinite(surprise):
                raise MemoryContractError("surprise overflow")
            if surprise < self.threshold:
                return False
            values = [(1 - self.decay) * a + self.decay * b
                      for a, b in zip(previous, values)]
            if not all(math.isfinite(x) for x in values):
                raise MemoryContractError("memory update overflow")
        self._entries[identity] = values
        self._entries.move_to_end(identity)
        while len(self._entries) > self.capacity:
            self._entries.popitem(last=False)
        return True

    def lookup(self, tenant: str, key: str) -> list[float] | None:
        if not _valid_id(tenant) or not _valid_id(key):
            raise MemoryContractError("invalid memory identity")
        result = self._entries.get((tenant, key))
        return result.copy() if result is not None else None

    def checkpoint(self) -> bytes:
        payload = {
            "version": 1, "dimension": self.dimension, "capacity": self.capacity,
            "threshold": self.threshold, "decay": self.decay,
            "entries": [[tenant, key, vector] for (tenant, key), vector in self._entries.items()],
        }
        raw = _canonical(payload)
        return _canonical({"payload": payload, "sha256": hashlib.sha256(raw).hexdigest()})

    @classmethod
    def restore(cls, raw: bytes) -> "SurpriseGatedMemory":
        if not isinstance(raw, bytes) or len(raw) > 16_000_000:
            raise MemoryContractError("invalid checkpoint size")
        try:
            envelope = json.loads(raw)
            if set(envelope) != {"payload", "sha256"}:
                raise MemoryContractError("invalid checkpoint envelope")
            payload = envelope["payload"]
            if hashlib.sha256(_canonical(payload)).hexdigest() != envelope["sha256"]:
                raise MemoryContractError("checkpoint checksum mismatch")
            if set(payload) != {"version", "dimension", "capacity", "threshold", "decay", "entries"} or payload["version"] != 1:
                raise MemoryContractError("unsupported checkpoint")
            result = cls(payload["dimension"], payload["capacity"],
                         payload["threshold"], payload["decay"])
            if not isinstance(payload["entries"], list) or len(payload["entries"]) > result.capacity:
                raise MemoryContractError("checkpoint capacity exceeded")
            for entry in payload["entries"]:
                if not isinstance(entry, list) or len(entry) != 3:
                    raise MemoryContractError("invalid checkpoint entry")
                tenant, key, vector = entry
                if not _valid_id(tenant) or not _valid_id(key):
                    raise MemoryContractError("invalid checkpoint identity")
                identity = (tenant, key)
                if identity in result._entries:
                    raise MemoryContractError("duplicate checkpoint identity")
                result._entries[identity] = result._vector(vector)
            return result
        except (TypeError, ValueError, KeyError, OverflowError) as error:
            raise MemoryContractError("invalid checkpoint") from error
