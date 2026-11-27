# AB-OM-01 / 0.51 — local GGUF file admission

Date: 2026-11-27. Base commit 1cbc9f1ace1ef9fc6d9cf6644e73143ccd6ec2d8. TESTED_LOCAL_PARTIAL.

Before: check_gguf opened a path before rejecting non-regular files; FIFO could block, a path replacement could change identity, and a truncated header could escape as struct.error. After: require regular lstat and size bounds before open; use O_NOFOLLOW/O_NONBLOCK when available, fstat identity/size/type verification, guaranteed descriptor close, explicit 24-byte header check.

Environment: Python 3.13.5 on Linux. Real filesystem objects and subprocess; deterministic race injection with patches around lstat/fstat; exact check_gguf section and minimal imports/RunnerError/regex harness, not full module import. Baseline blob f564bd6aaabd6d09a4045d3b450d453d6fcd2378; baseline section SHA-1 ca1dbef2be762bd40e56adee9cd1cf0b19d06c44; tested fixed section 2886f0a8fee68863699f6cb8dac5ab59657b59c2; test blob 9da21f46ba2cf14f74c7ffc03c78e3ac6a5f5c07; patched full source blob 9d5ca79b93296c756423e53b1fb6603795feef55.

Command: python3 -m unittest discover -s scripts/tests -p test_local_runner_file_admission.py -v.
RED: 2 PASS, 1 FAIL, 2 ERROR, exit 1. FIFO child terminated after 3-second test timeout; truncated header raised struct.error; replacement accepted. RED log SHA-256 e41c69da4209fe32ff453adf34b34b39f03b1d354dba79014373f5fc043e04bc.
GREEN: 5/5 PASS, 0 skipped, exit 0. GREEN log SHA-256 bca5afe8167ca99a4aab5c5ce75ecd2fe0275ee8354b7c1509dca5bacbcabb37.
Local reconstructed module py_compile exit 0. Logs local, not committed. Existing admission test discovery wildcard covers new test; workflow path filter extended. No manual CI dispatch.

Limits: the later llama-server launch independently reopens the model pathname; this change does not prove end-to-end TOCTOU freedom. O_NOFOLLOW and O_NONBLOCK fallback platforms not tested; actual llama model execution, full module/repository regression, Rust build and production remain unverified. No acceptance status promotion.
