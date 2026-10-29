; OCAF stage 334: symbolic uniqueness constraint of durable nonce ledger.
; UNSAT if two successful admissions use same (principal,nonce) identity.
(set-logic QF_LIA)
(declare-const principal_1 Int)
(declare-const principal_2 Int)
(declare-const nonce_1 Int)
(declare-const nonce_2 Int)
(assert (= principal_1 principal_2))
(assert (= nonce_1 nonce_2))
; Unique-key admission requires distinct composite identities.
(assert (or (not (= principal_1 principal_2))
            (not (= nonce_1 nonce_2))))
(check-sat)
