# 0.39.25 — wspólna próba Aya i awarii procesu podpisującego

[Run 40202208778](https://github.com/mojealterego/ocaf-runtime/actions/runs/40202208778)
na commicie `f8c0a84` zakończył się sukcesem na rzeczywistym hoście Linux/KVM.
Artefakt `12601169916` ma SHA-256 archiwum
`2f887ba7a80ef7cadadd7ac722bcdd09fda26d92068c9285cbb3d15cd8fa5045`.
Wymagany krok `ACTUAL guest Aya and host Aya remain live while separate broker
signer is SIGKILLed` przeszedł.

`scripts/stage039_joint_fault_guest_aya.py --fault-mode broker-sigkill`
uruchamia dwie jailed microVM. Hostowy link Aya jest aktywny przed zwolnieniem
PID1 gościa A. W gościu A sonda Aya otrzymuje dodatni kernel BPF program ID
i zdarzenia `openat` procesu Wasmtime, po czym PID1 wysyła przez AF_VSOCK
powiązane z VM wyzwanie. Oddzielny proces podpisujący przyjmuje poprawne
wyzwanie, lecz przed wydaniem permit zostaje rzeczywiście zabity przez
`SIGKILL`. Po zamknięciu połączenia gość A zgłasza
`OCAF039_Q_GUEST_DENIED=permit_receive`; drugi jailed tenant pozostaje
uruchomiony. Bramka wymaga także braku wydanego permit oraz odzyskania
hostowej sondy Aya po zatrzymaniu VM.

To dowód odmowy przy awarii **laboratoryjnego procesu podpisującego** w tym
samym hoście. Nie dowodzi failover trwałego brokera, dzierżawy między hostami
ani partycji sieci w tej samej próbie. Klucz procesu i podpis manifestu są
laboratoryjne. `stage_039_complete=false`, `production_approved=false`.
