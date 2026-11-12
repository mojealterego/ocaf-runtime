"""Unprivileged NOEVARIS host-side adapter for TRS v1 JSON wire contract.

Transport is injected by a trusted host. This class performs NO authentication.
"""
import json

_OPERATIONS = frozenset(('trs.health', 'trs.geometry.fisher_rao', 'trs.geometry.hodge_laplacian', 'trs.math.sat_evaluate'))
_MAX_PAYLOAD = 65536


class BridgeContractError(ValueError):
    pass


def _identity(value):
    return type(value) is str and 0 < len(value.encode('utf-8')) <= 256 and not any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in value)


class BridgeClient:
    """Transport callback accepts a request dict and returns a response dict.

    The trusted host is responsible for transport authentication, fencing,
    capabilities, replay and authorization; this is only a wire codec.
    """
    def __init__(self, transport):
        if not callable(transport):
            raise TypeError('transport must be callable')
        self._transport = transport

    def call(self, *, tenant, sender, operation, arguments):
        if not _identity(tenant) or not _identity(sender) or type(operation) is not str or operation not in _OPERATIONS:
            raise BridgeContractError('INVALID_REQUEST_IDENTITY_OR_OPERATION')
        if type(arguments) is not dict:
            raise BridgeContractError('INVALID_ARGUMENTS')
        if operation == 'trs.health':
            if arguments:
                raise BridgeContractError('INVALID_ARGUMENTS')
            payload = b''
        else:
            try:
                payload = json.dumps(arguments, separators=(',', ':'), allow_nan=False).encode('utf-8')
            except (TypeError, ValueError, OverflowError) as exc:
                raise BridgeContractError('INVALID_ARGUMENTS') from exc
        if len(payload) > _MAX_PAYLOAD:
            raise BridgeContractError('PAYLOAD_TOO_LARGE')
        request = {'version': 1, 'tenant': tenant, 'sender': sender, 'operation': operation, 'payload': list(payload)}
        response = self._transport(request)
        if type(response) is not dict or set(response) != {'version', 'accepted', 'payload'}:
            raise BridgeContractError('INVALID_RESPONSE')
        if type(response['version']) is not int or response['version'] != 1 or type(response['accepted']) is not bool or not response['accepted']:
            raise BridgeContractError('REJECTED_OR_UNSUPPORTED_RESPONSE')
        data = response['payload']
        if type(data) is not list or len(data) > _MAX_PAYLOAD or any(type(v) is not int or v < 0 or v > 255 for v in data):
            raise BridgeContractError('INVALID_RESPONSE_PAYLOAD')
        try:
            decoded = json.loads(bytes(data))
        except (ValueError, UnicodeDecodeError) as exc:
            raise BridgeContractError('INVALID_RESPONSE_JSON') from exc
        if type(decoded) is not dict:
            raise BridgeContractError('INVALID_RESPONSE_JSON')
        return decoded
