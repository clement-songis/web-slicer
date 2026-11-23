# Guide utilisateur — web-slicer

web-slicer est un trancheur 3D dans le navigateur, en parité avec
OrcaSlicer : mêmes paramètres, mêmes presets système, même G-code.
Chaque compte voit ses propres projets (isolation stricte).

## Démarrer

1. Ouvrir l'application (en local : http://127.0.0.1:15173).
2. Créer un compte sur /register. **Le premier compte devient
   administrateur** ; selon la politique configurée, les inscriptions
   suivantes peuvent être fermées (le compte admin invite alors les
   utilisateurs depuis /admin).
3. Les presets système (imprimantes, filaments, processus) sont déjà
   chargés — le catalogue permet de choisir un profil comme dans Orca.

## Préparer une impression

1. **Projets** → nouveau projet, puis importer un modèle
   (STL, OBJ, 3MF, STEP).
2. La **scène 3D** permet de placer, tourner, grouper, découper (Cut),
   simplifier et peindre (support, couture, peau floue) chaque objet,
   comme dans Orca.
3. Les **réglages** reprennent l'organisation d'Orca (onglets et
   sections) ; les surcharges par objet et par plateau sont possibles.
4. **Trancher** produit le G-code avec l'aperçu par couche
   ( lignes, vitesses, purges, hauteur de couche variable ).

## Exporter / imprimer

- **Export 3MF** : projet compatible Orca ( réglages, plateaux,
  surcharges relisibles ).
- **Export G-code** : téléchargement direct du fichier tranché.
- **Envoyer à l'imprimante** : imprimantes Klipper/Moonraker déclarées
  dans /printers ( URL du serveur ) ; envoi avec lancement immédiat,
  statut et contrôles ( pause, reprise, annulation ). Une imprimante
  injoignable renvoie une erreur explicite, jamais un silence.

## File d'attente et suivi

Les tranchages s'enchaînent dans la file (/queue) ; un crash du moteur
C++ est contenu dans un sous-processus — la requête échoue proprement,
les autres continuent.
