#!/usr/bin/env python3
"""SOURCE-1.65 deterministic hybrid retrieval *reference model*, not Rust.

This bounded, in-memory model checks BM25 + cosine reciprocal-rank fusion,
tenant isolation, optimistic revision fencing and stale posting invalidation.
Embeddings are supplied by a trusted caller, never generated or authenticated
here. No provider, persistence, signing, or deployment is claimed.
"""
from __future__ import annotations
from collections import Counter
from dataclasses import dataclass
import hashlib
import math
import re

_MAX_DOCS = 256
_MAX_BODY_BYTES = 65536
_MAX_TOTAL_BYTES = 2 * 1024 * 1024
_MAX_EMBEDDING = 256
_MAX_QUERY_BYTES = 2048
_MAX_QUERY_TOKENS = 256
_MAX_RESULTS = 100


class HybridError(ValueError):
    """Reject invalid, stale, or unauthorized reference-model operations."""


@dataclass(frozen=True)
class HybridHit:
    source_id: str
    rrf_score: float
    lexical_rank: int | None
    semantic_rank: int


def _valid_id(s: str) -> bool:
    return isinstance(s, str) and 0 < len(s) <= 128 and all(
        (c.isascii() and (c.isalnum() or c in '_.:-')) for c in s)


def _tokens(text: str) -> list[str]:
    return ''.join(c if c.isalnum() else ' ' for c in text).lower().split()


def _vector(values: list[float] | tuple[float, ...], dimension: int) -> tuple[float, ...]:
    if not isinstance(values, (list, tuple)) or len(values) != dimension:
        raise HybridError('embedding dimension mismatch')
    if not all(type(x) in (float, int) and math.isfinite(x) for x in values):
        raise HybridError('nonfinite embedding')
    vec = tuple(float(x) for x in values)
    if not math.isfinite(math.fsum(x * x for x in vec)) or math.fsum(x * x for x in vec) <= 1e-24:
        raise HybridError('embedding norm invalid')
    return vec


class HybridReference:
    """Tenant-bound reference implementation; Rust code is authoritative."""

    def __init__(self, authenticated_tenant: str, model_id: str, dimension: int) -> None:
        if not _valid_id(authenticated_tenant) or not _valid_id(model_id) or type(dimension) is not int or not 1 <= dimension <= _MAX_EMBEDDING:
            raise HybridError('invalid index configuration')
        self.tenant = authenticated_tenant
        self.model = model_id
        self.dimension = dimension
        self.revision = 0
        self._records: dict[str, tuple[str, tuple[float, ...]]] = {}

    def _tenant(self, authenticated_tenant: str) -> None:
        if not _valid_id(authenticated_tenant) or authenticated_tenant != self.tenant:
            raise HybridError('authenticated tenant mismatch')

    def upsert(self, authenticated_tenant: str, source_id: str, body: str,
               embedding: list[float] | tuple[float, ...], model_id: str,
               source_sha256: str, expected_revision: int) -> int:
        self._tenant(authenticated_tenant)
        if not _valid_id(source_id) or not isinstance(body, str) or not isinstance(source_sha256, str):
            raise HybridError('invalid source')
        if type(expected_revision) is not int or expected_revision != self.revision:
            raise HybridError('stale revision')
        if model_id != self.model:
            raise HybridError('embedding model mismatch')
        body_bytes = body.encode('utf-8')
        if len(body_bytes) > _MAX_BODY_BYTES or len(_tokens(body)) > 10000:
            raise HybridError('source limit exceeded')
        if hashlib.sha256(body_bytes).hexdigest() != source_sha256:
            raise HybridError('embedding/source digest mismatch')
        checked_embedding = _vector(embedding, self.dimension)
        candidate = self._records.copy()
        candidate[source_id] = (body, checked_embedding)
        return self._publish(candidate)

    def remove(self, authenticated_tenant: str, source_id: str, expected_revision: int) -> int:
        self._tenant(authenticated_tenant)
        if not _valid_id(source_id) or type(expected_revision) is not int or expected_revision != self.revision or source_id not in self._records:
            raise HybridError('stale or absent document')
        candidate = self._records.copy()
        del candidate[source_id]
        return self._publish(candidate)

    def _publish(self, candidate: dict[str, tuple[str, tuple[float, ...]]]) -> int:
        if len(candidate) > _MAX_DOCS or sum(len(body.encode('utf-8')) for body, _ in candidate.values()) > _MAX_TOTAL_BYTES:
            raise HybridError('index memory limit exceeded')
        if self.revision >= (1 << 64) - 1:
            raise HybridError('revision exhausted')
        self._records = candidate
        self.revision += 1
        return self.revision

    def search(self, authenticated_tenant: str, query: str,
               query_embedding: list[float] | tuple[float, ...], limit: int) -> list[HybridHit]:
        self._tenant(authenticated_tenant)
        if not isinstance(query, str) or not query or len(query.encode('utf-8')) > _MAX_QUERY_BYTES or type(limit) is not int or not 1 <= limit <= _MAX_RESULTS:
            raise HybridError('invalid query or limit')
        tokens = _tokens(query)
        if not tokens or len(tokens) > _MAX_QUERY_TOKENS:
            raise HybridError('invalid query tokens')
        q = _vector(query_embedding, self.dimension)
        if not self._records:
            return []
        # Implement the exact BM25 constants and tie-break used by LexicalIndex.
        docs = sorted(self._records.items())
        frequency = {key: Counter(_tokens(body)) for key, (body, _) in docs}
        lengths = {key: sum(frequency[key].values()) for key, _ in docs}
        mean_length = sum(lengths.values()) / len(docs)
        lexical_scores: dict[str, float] = {}
        for term in set(tokens):
            df = sum(1 for counts in frequency.values() if counts[term] > 0)
            if not df:
                continue
            idf = math.log((len(docs) - df + 0.5) / (df + 0.5) + 1.0)
            for source_id, _ in docs:
                tf = frequency[source_id][term]
                if not tf:
                    continue
                length_factor = 1.0 if mean_length == 0 else 0.25 + 0.75 * (lengths[source_id] / mean_length)
                lexical_scores[source_id] = lexical_scores.get(source_id, 0.0) + idf * (tf * 2.2) / (tf + 1.2 * length_factor)
        lexical = sorted(((source_id, score) for source_id, score in lexical_scores.items()
                          if score > 0 and math.isfinite(score)), key=lambda v: (-v[1], v[0]))[:100]
        lex_ranks = {source_id: rank for rank, (source_id, _) in enumerate(lexical, 1)}
        q_norm = math.sqrt(math.fsum(v * v for v in q))
        semantic = []
        for source_id, (_, embedding) in docs:
            norm = math.sqrt(math.fsum(v * v for v in embedding))
            cosine = math.fsum(a * b for a, b in zip(q, embedding)) / (q_norm * norm)
            semantic.append((source_id, cosine))
        semantic.sort(key=lambda v: (-v[1], v[0]))
        sem_ranks = {source_id: rank for rank, (source_id, _) in enumerate(semantic, 1)}
        hits = [HybridHit(source_id, (1 / (60 + sem_rank)) + (1 / (60 + lex_ranks[source_id]) if source_id in lex_ranks else 0),
                          lex_ranks.get(source_id), sem_rank)
                for source_id, sem_rank in sem_ranks.items()]
        hits.sort(key=lambda hit: (-hit.rrf_score, hit.source_id))
        return hits[:limit]
