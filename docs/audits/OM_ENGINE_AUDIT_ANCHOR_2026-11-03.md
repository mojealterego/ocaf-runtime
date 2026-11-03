# OM Engine — audit anchor / high-water mark

Data: 2026-11-03. Etap: OCAF 0.56 (bez nowego numeru i certyfikacji).

## Zakres kodu
- `src/om_engine.rs`: serializowalny `AuditHead`, tenant/workflow binding, `verify_audit_anchor`.
- `tests/om_engine.rs`: weryfikacja serializacji, odrzucenia cudzego checkpointu i odrzucenia obciętego dziennika z pozornie poprawnym prefiksem.
- `docs/om-engine/ARCHITECTURE.md`: kontrakt zewnętrznego przypięcia i ograniczenia TOCTOU.

## Granica zaufania
Lokalny journal SHA-256 nie broni przed administratorem bazy: można usunąć sufiks albo odtworzyć całość. Odpowiednio chroniony `AuditHead` z **innej domeny zaufania** umożliwia jedynie wykrycie rozbieżności. Brak automatycznego podpisania, atestacji, remote storage, transakcyjnego warunku pin przy mutacji oraz produkcyjnego connectora Digital-Life. Weryfikator nie może przyjmować `AuditHead` jako dowodu, jeśli model lub nieautoryzowany caller go dostarczył.

## Testy
RED: commit `dd1a6b7135290a5b7abb7855abbbea56389677c9`, GitHub Actions OM Engine #41585640213 `failure` przed implementacją.
GREEN: [GitHub Actions OM Engine #41586244060](https://github.com/mojealterego/ocaf-runtime/actions/runs/41586244060), `completed/success` na `c62e0f013cb355bb02dca6ac90b16e71e3e56088` — 6/6 testów Rust PASS, rustfmt PASS i Clippy PASS. To nie jest odbiór produkcyjny. Równoległy workflow Rust CI #41586244041 zakończył się `failure` na kroku `Offline release precheck unit tests`, przed kompilacją Rust.
