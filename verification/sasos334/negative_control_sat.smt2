; OCAF stage 334: mandatory SAT negative control to detect a broken gate.
(set-logic QF_LIA)
(declare-const tenant Int)
(assert (= tenant 7))
(check-sat)
