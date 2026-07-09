# Contrat — trait `Storage` (backend/domain)

Constitution III : le métier ne connaît ni SQL ni le SGBD. Implémentations :
`adapters/storage/sqlite` (défaut), `adapters/storage/postgres` (feature
`postgres`, sélection runtime par `DATABASE_URL`). Entités : voir
[data-model.md](../data-model.md).

## Traits

```rust
pub trait Storage: Send + Sync {
    fn users(&self) -> &dyn UserRepo;
    fn projects(&self) -> &dyn ProjectRepo;
    fn models(&self) -> &dyn ModelRepo;
    fn presets(&self) -> &dyn PresetRepo;
    fn printers(&self) -> &dyn PrinterRepo;
    fn jobs(&self) -> &dyn JobRepo;
    fn gcodes(&self) -> &dyn GcodeRepo;
    fn instance(&self) -> &dyn InstanceRepo; // réglages d'instance, invitations
}
```

Chaque repo expose des opérations métier (pas du CRUD générique), toutes
**scopées par `user_id`** quand la ressource est possédée — l'oubli du scope
doit être impossible par signature (ex.
`fn get(&self, owner: UserId, id: ProjectId)`). Exemples notables :

- `PresetRepo::list_compatible(kind, printer_name, user_id)` — presets système
  + presets de l'utilisateur, filtrés `instantiation=true` et compatibilité (FR-021).
- `JobRepo::claim_next()` — réclamation transactionnelle d'un job `queued`
  (un seul worker gagne) ; `JobRepo::requeue_running()` — reprise au boot (R9).
- `UserRepo::create_first_admin_if_empty()` — bootstrap d'instance.

## Garanties (suite de tests de contrat générique)

La même suite `storage_contract_tests(s: &dyn Storage)` doit passer sur les
deux backends :

1. Isolation : une ressource créée par A est invisible/inaccessible pour B
   (get, list, delete) — support de SC-008.
2. Unicité : email (users), nom de projet par utilisateur, nom de preset par
   (kind, origin, user).
3. Transactions : `claim_next` ne délivre jamais deux fois le même job sous
   concurrence (test multi-tâches).
4. Cascade : suppression compte → projets, modèles, presets user, jobs,
   gcodes, imprimantes supprimés (la purge filesystem relève de
   `adapters/files`, orchestrée par le domaine).
5. Presets système re-seedables sans toucher aux presets utilisateur.

## Migrations

`backend/migrations/sqlite/` et `backend/migrations/postgres/` maintenues en
parallèle ; un test vérifie que les deux schémas acceptent la même suite de
contrat. Aucune migration de code métier pour changer de backend
(constitution III).
