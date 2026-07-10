//! Identifiants typés (UUID v4) — un type par entité pour rendre impossible
//! le mélange d'IDs entre ressources (ex. passer un `ProjectId` là où un
//! `UserId` est attendu ne compile pas).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Déclare un newtype d'identifiant au-dessus d'`Uuid`.
macro_rules! typed_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Nouvel identifiant aléatoire.
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<Uuid> for $name {
            fn from(id: Uuid) -> Self {
                Self(id)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Uuid {
                id.0
            }
        }
    };
}

typed_id!(
    /// Identifiant d'utilisateur.
    UserId
);
typed_id!(
    /// Identifiant de projet.
    ProjectId
);
typed_id!(
    /// Identifiant de modèle 3D importé.
    ModelId
);
typed_id!(
    /// Identifiant de preset.
    PresetId
);
typed_id!(
    /// Identifiant d'imprimante déclarée.
    PrinterId
);
typed_id!(
    /// Identifiant de job de tranchage.
    JobId
);
typed_id!(
    /// Identifiant de G-code produit.
    GcodeId
);
typed_id!(
    /// Identifiant d'invitation.
    InvitationId
);
