# OMEGA OCAF APEX integration (main)

This directory contains a repository-owned validation contract for the OMEGA OCAF APEX plugin. It does not imply the plugin's runtime is deployed into this repository.

## Evidence levels
- `validate.py`: static integration contract and Python syntax; no cloud dependencies.
- `openai_probe.py`: performs one real OpenAI Responses request only when an authorized `OPENAI_API_KEY` is available.
- GitHub Actions `omega-ocaf-apex-e2e.yml`: automatically validates this contract on main and supports a manual workflow dispatch.
- Absence of an API key is **NOT VERIFIED**, not a successful inference test.

## Safety
Use repository secrets for credentials. Never commit secrets. This workflow does not authorize GitHub write access or cloud deployment. Production E2E requires a separately authorized live write/readback, cloud deployment, and independent operational evidence.
