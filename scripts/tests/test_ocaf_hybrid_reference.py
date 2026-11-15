"""Engineering reference tests for SOURCE-1.65's trust-bounded hybrid ranking."""
import hashlib
import math
import unittest

from scripts.ocaf_hybrid_reference import HybridReference, HybridError


def digest(body: str) -> str:
    return hashlib.sha256(body.encode('utf-8')).hexdigest()


class HybridReferenceTests(unittest.TestCase):
    def setUp(self):
        self.index = HybridReference('tenant_a', 'emb_v1', 2)

    def put(self, id: str, body: str, vector, revision=None, tenant='tenant_a', model='emb_v1', sha=None):
        return self.index.upsert(tenant, id, body, vector, model,
                                 digest(body) if sha is None else sha,
                                 self.index.revision if revision is None else revision)

    def test_semantic_search_without_lexical_match(self):
        self.put('a', 'forest', [1, 0])
        self.put('b', 'desert', [0, 1])
        result = self.index.search('tenant_a', 'unmatched', [1, 0], 2)
        self.assertEqual([r.source_id for r in result], ['a', 'b'])
        self.assertIsNone(result[0].lexical_rank)
        self.assertEqual(result[0].semantic_rank, 1)

    def test_rrf_mixes_real_lexical_and_semantic_rankings(self):
        self.put('vector', 'alpha', [1, 0])
        self.put('lexical', 'needle needle needle', [0, 1])
        ranks = self.index.search('tenant_a', 'needle', [1, 0], 2)
        self.assertEqual(ranks[0].source_id, 'lexical')
        self.assertEqual(ranks[0].lexical_rank, 1)
        self.assertEqual(ranks[0].semantic_rank, 2)
        self.assertEqual(ranks[1].source_id, 'vector')
        self.assertIsNone(ranks[1].lexical_rank)
        self.assertEqual(ranks[1].semantic_rank, 1)

    def test_upsert_replaces_stale_content_and_vector_together(self):
        self.put('doc', 'stale secret', [0, 1])
        self.assertIsNotNone(self.index.search('tenant_a', 'stale', [1, 0], 1)[0].lexical_rank)
        self.put('doc', 'fresh', [1, 0])
        x = self.index.search('tenant_a', 'stale', [1, 0], 1)
        self.assertIsNone(x[0].lexical_rank)
        self.assertEqual(x[0].semantic_rank, 1)
        self.assertEqual(self.index.revision, 2)

    def test_cross_tenant_access_fails_closed(self):
        self.put('id', 'secret', [1, 0])
        with self.assertRaises(HybridError):
            self.index.search('tenant_b', 'secret', [1, 0], 1)
        with self.assertRaises(HybridError):
            self.put('cross', 'secret', [1, 0], tenant='tenant_b')
        with self.assertRaises(HybridError):
            self.index.remove('tenant_b', 'id', 1)
        self.assertEqual(self.index.revision, 1)

    def test_bad_digest_or_model_never_publish(self):
        for kwargs in ({'sha': '0' * 64}, {'model': 'different'}):
            with self.subTest(kwargs=kwargs), self.assertRaises(HybridError):
                self.put('doc', 'body', [1, 0], **kwargs)
        self.assertEqual(self.index.revision, 0)
        self.assertEqual(self.index.search('tenant_a', 'body', [1, 0], 1), [])

    def test_stale_revision_and_invalid_vector_do_not_mutate(self):
        self.put('ok', 'before', [1, 0])
        bad_vectors = ([0, 0], [float('nan'), 1], [1], [math.inf, 0])
        for values in bad_vectors:
            with self.subTest(values=values), self.assertRaises(HybridError):
                self.put('ok', 'after', values)
        with self.assertRaises(HybridError):
            self.put('ok', 'after', [1, 0], revision=0)
        self.assertEqual(self.index.revision, 1)
        self.assertIsNotNone(self.index.search('tenant_a', 'before', [1, 0], 1)[0].lexical_rank)

    def test_delete_erases_doc_and_stale_replays_fail(self):
        self.put('a', 'present', [1, 0])
        self.assertEqual(self.index.remove('tenant_a', 'a', 1), 2)
        self.assertEqual(self.index.search('tenant_a', 'present', [1, 0], 3), [])
        with self.assertRaises(HybridError):
            self.index.remove('tenant_a', 'a', 2)
        with self.assertRaises(HybridError):
            self.put('a', 'resurrection', [1, 0], revision=1)
        self.assertEqual(self.index.revision, 2)

    def test_deterministic_tie_break_by_source_identifier(self):
        self.put('zeta', 'same', [1, 0])
        self.put('alpha', 'same', [1, 0])
        first = self.index.search('tenant_a', 'same', [1, 0], 2)
        second = self.index.search('tenant_a', 'same', [1, 0], 2)
        self.assertEqual(first, second)
        self.assertEqual([x.source_id for x in first], ['alpha', 'zeta'])

    def test_limits_and_invalid_query(self):
        with self.assertRaises(HybridError):
            HybridReference('a', 'm', 0)
        with self.assertRaises(HybridError):
            HybridReference('a', 'm', 257)
        with self.assertRaises(HybridError):
            self.put('x', 'x' * 65537, [1, 0])
        self.put('x', 'short', [1, 0])
        for query, vector, limit in [('', [1, 0], 1), ('!!!', [1, 0], 1), ('a', [0, 0], 1), ('a', [1, 0], 101)]:
            with self.subTest(query=query, vector=vector, limit=limit), self.assertRaises(HybridError):
                self.index.search('tenant_a', query, vector, limit)
        self.assertEqual(self.index.revision, 1)

    def test_document_capacity_limit_is_atomic(self):
        for i in range(256):
            self.put(f'doc{i}', 'a', [1, 0])
        self.assertEqual(self.index.revision, 256)
        with self.assertRaises(HybridError):
            self.put('extra', 'b', [1, 0])
        self.assertEqual(self.index.revision, 256)
        self.assertEqual(len(self.index.search('tenant_a', 'a', [1, 0], 100)), 100)


if __name__ == '__main__':
    unittest.main()
