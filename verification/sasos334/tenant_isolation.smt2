; OCAF stage 334: bounded symbolic model of tenant and subject authorization.
; UNSAT means no cross-tenant execution in this model, NOT a proof of Rust.
(set-logic QF_LIA)
(declare-const issuer_tenant Int)
(declare-const peer_tenant Int)
(declare-const requested_tenant Int)
(declare-const issuer_subject Int)
(declare-const peer_subject Int)
(assert (= issuer_tenant 1))
(assert (= issuer_subject 17))
(assert (= peer_tenant 2))
(assert (= requested_tenant peer_tenant))
(assert (= peer_subject issuer_subject))
; Authorization requires both tenant and subject match.
(assert (= issuer_tenant peer_tenant))
(assert (= issuer_subject peer_subject))
(check-sat)
