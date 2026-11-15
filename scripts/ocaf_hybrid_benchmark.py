#!/usr/bin/env python3
"""SOURCE-2.05: bounded, measured local hybrid search reference workload.

Benchmarks an actual Python SOURCE-1.65 reference implementation (BM25,
cosine and RRF) on a deterministic corpus. It does not benchmark native Rust,
an external model, multi-host architecture, or production hardware. All
provider/model claims are explicitly excluded.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import random
import sys
import time
from typing import Callable

if __package__ in (None, ''):
    # Direct CLI: the script directory, not the repo root, is on sys.path.
    from ocaf_benchmark_harness import (
        BenchmarkConfig, BenchmarkError, environment, measure,
    )
    from ocaf_hybrid_reference import HybridReference, HybridError
else:
    from scripts.ocaf_benchmark_harness import (
        BenchmarkConfig, BenchmarkError, environment, measure,
    )
    from scripts.ocaf_hybrid_reference import HybridReference, HybridError

TENANT = 'benchmark_tenant'
MODEL = 'engineering_reference_v1'
MAX_BENCH_SAMPLES = 2000
MAX_BENCH_WARMUPS = 200
MAX_BENCH_DOCS = 256
MAX_BENCH_DIM = 32
MAX_BENCH_QUERIES = 32


def _validate(documents: int, dimension: int, seed: int, queries: int,
              samples: int, warmups: int, profile: str) -> None:
    if (
        type(documents) is not int or not 1 <= documents <= MAX_BENCH_DOCS
        or type(dimension) is not int or not 1 <= dimension <= MAX_BENCH_DIM
        or type(seed) is not int or not 0 <= seed < 2**32
        or type(queries) is not int or not 1 <= queries <= min(documents, MAX_BENCH_QUERIES)
        or type(samples) is not int or not 1 <= samples <= MAX_BENCH_SAMPLES
        or type(warmups) is not int or not 0 <= warmups <= MAX_BENCH_WARMUPS
        or profile not in ('warm', 'cold')
        or (profile == 'cold' and warmups != 0)
    ):
        raise BenchmarkError('invalid bounded SOURCE-2.05 hybrid workload')


def build_workload(documents: int, dimension: int, seed: int, queries: int):
    """Build deterministic fixture and return its exact corpus SHA-256.

    The seed changes insertion/query order, never the corpus contents.
    """
    _validate(documents, dimension, seed, queries, 1, 0, 'warm')
    rows = []
    for i in range(documents):
        vector = [0.0] * dimension
        vector[i % dimension] = 1.0
        rows.append({
            'source_id': f'doc_{i:04d}',
            'body': f'anchor token{i:04d} corpus',
            'vector': vector,
        })
    corpus_raw = json.dumps(rows, sort_keys=True, separators=(',', ':')).encode('utf-8')
    corpus_sha256 = hashlib.sha256(corpus_raw).hexdigest()
    rng = random.Random(seed)
    shuffled = list(range(documents))
    rng.shuffle(shuffled)
    index = HybridReference(TENANT, MODEL, dimension)
    for i in shuffled:
        row = rows[i]
        text = row['body']
        index.upsert(TENANT, row['source_id'], text, row['vector'], MODEL,
                     hashlib.sha256(text.encode('utf-8')).hexdigest(), index.revision)
    selected = rng.sample(range(documents), queries)
    tasks = [(f'token{i:04d}', rows[i]['vector'], rows[i]['source_id']) for i in selected]
    return index, tasks, corpus_sha256, len(corpus_raw)


def run_benchmark(*, documents: int = 32, dimension: int = 8, seed: int = 0,
                  queries: int = 8, samples: int = 100, warmups: int = 10,
                  profile: str = 'warm', clock: Callable[[], int] = time.perf_counter_ns) -> dict:
    """Measure real reference queries; fail if any ranked answer is incorrect.

    The timed callback includes correctness verification, so reported latency
    is the combined search + top-1 check, not pure search kernel time.
    """
    _validate(documents, dimension, seed, queries, samples, warmups, profile)
    index, tasks, corpus_sha256, total_bytes = build_workload(documents, dimension, seed, queries)
    if total_bytes > 1_048_576:
        raise BenchmarkError('workload source exceeds bounded payload')
    config = BenchmarkConfig(
        name='hybrid_bm25_cosine_rrf_reference',
        profile=profile,
        warmups=warmups,
        samples=samples,
        payload_bytes=total_bytes,
    )

    def verify_query(n: int) -> None:
        query, embedding, relevant = tasks[n % len(tasks)]
        try:
            hits = index.search(TENANT, query, embedding, 1)
        except HybridError as exc:
            raise BenchmarkError('hybrid reference search failed') from exc
        if len(hits) != 1 or hits[0].source_id != relevant:
            raise BenchmarkError('reference quality gate rejected top-1 result')

    stats = measure(config, verify_query, clock=clock)
    return {
        **stats,
        'status': 'MEASURED_LOCAL_PYTHON_REFERENCE_ONLY',
        'requirement_id': 'SOURCE-2.05',
        'workload_source': 'SOURCE-1.65',
        'search_algorithm': 'BM25_COSINE_RRF_K60',
        'measured_operation': 'search_plus_top1_verification',
        'candidate_documents': documents,
        'embedding_dimension': dimension,
        'query_count': len(tasks),
        'deterministic_seed': seed,
        'corpus_sha256': corpus_sha256,
        'verified_top1_samples': samples,
        'native_rust_verified': False,
        'external_vector_provider_verified': False,
        'environment_attested': False,
        'production_approved': False,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, required=True)
    parser.add_argument('--documents', type=int, default=32)
    parser.add_argument('--dimension', type=int, default=8)
    parser.add_argument('--seed', type=int, default=0)
    parser.add_argument('--queries', type=int, default=8)
    parser.add_argument('--samples', type=int, default=100)
    parser.add_argument('--warmups', type=int, default=10)
    parser.add_argument('--profile', choices=('warm', 'cold'), default='warm')
    args = parser.parse_args(argv)
    try:
        _validate(args.documents, args.dimension, args.seed, args.queries,
                  args.samples, args.warmups, args.profile)
        provenance = environment(args.repo)
        metrics = run_benchmark(documents=args.documents, dimension=args.dimension,
                                seed=args.seed, queries=args.queries,
                                samples=args.samples, warmups=args.warmups,
                                profile=args.profile)
    except BenchmarkError as exc:
        print(json.dumps({'status': 'INVALID_BENCHMARK', 'production_approved': False,
                          'error': str(exc)}, sort_keys=True))
        return 2
    print(json.dumps({'environment': provenance, 'benchmark': metrics}, sort_keys=True))
    return 0


if __name__ == '__main__':
    sys.exit(main())
