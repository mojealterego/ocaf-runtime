# Reproducible PostgreSQL 16 lab runtime; this uses one host kernel.
FROM postgres:16-bookworm@sha256:0ea6700a3b4f0ae6ce746519073558aed4d88a79d8d07622a9a644946c7319c4
RUN apt-get update \
    && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*
COPY --chown=postgres:postgres scripts/stage040_postgres_ha.py scripts/stage040_quorum.py /work/scripts/
USER postgres
ENTRYPOINT ["python3", "/work/scripts/stage040_postgres_ha.py"]
