# OMAt Contract Guard — transport exemption rationale

This verifier operates only against a local filesystem and exits after an offline deterministic validation. It neither subscribes nor publishes distributed messages; therefore its artifact transport classification is none. Zenoh remains mandatory for the actual OCAF agents, swarm coordinators, OMAt network tools and OMAs servers when their approved architecture requires distributed communication. This local exception cannot be reused for networked components.
