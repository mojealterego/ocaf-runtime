# Vault Transit least-privilege service ACL. Key creation/rotation require separate operator role.
path "transit/encrypt/ocaf-stage044" { capabilities = ["update"] }
path "transit/decrypt/ocaf-stage044" { capabilities = ["update"] }
path "transit/rewrap/ocaf-stage044" { capabilities = ["update"] }
