# Get your Deezer ARL (2 minutes)

The **ARL** is Deezer's session cookie. It lets Breezer authenticate as your account.
It is stored **locally** in your config directory (`~/.config/breezer/config.json`) and never leaves your machine.

> ⚠️ The ARL is a **secret**. Anyone holding it can access your Deezer account. Do not share it, do not paste it in issues/PRs.

## Linux / macOS / Windows — same procedure

1. Open **https://www.deezer.com** in your browser (Chrome, Firefox, Edge…).
2. Log in with your account.
3. Open the **developer tools**:
   - Chrome/Edge: `Ctrl+Shift+I` (Windows/Linux) or `Cmd+Alt+I` (macOS)
   - Firefox: `Ctrl+Shift+I` / `Cmd+Alt+I`
4. Go to the **Application** tab (Chrome/Edge) → **Cookies** → `www.deezer.com`
   (Firefox: **Storage** → **Cookies**).
5. Find the cookie named `arl`, copy its **Value** (a long hexadecimal string, ~192 chars).
6. In Breezer: open the sidebar → *Connexion* → paste the ARL → **Se connecter**.

Breezer then validates the ARL against Deezer and fetches your API token automatically.

## Troubleshooting

| Problem | Fix |
|---|---|
| `Connexion refusée` | The ARL expired or was regenerated — copy it again from the browser. |
| The login page won't open | Click *Ouvrir le navigateur* manually, or copy the Deezer URL into your browser. |
| You get disconnected | ARLs rotate; re-paste a fresh one. We cache the fresh token per session. |

---

# Obtenez votre ARL Deezer (2 minutes)

L'**ARL** est le cookie de session Deezer. Il permet à Breezer de s'authentifier avec votre compte.
Il est stocké **localement** dans votre dossier de config (`~/.config/breezer/config.json`) et ne quitte jamais votre machine.

> ⚠️ L'ARL est un **secret**. Quiconque le détient peut accéder à votre compte Deezer. Ne le partagez pas, ne le collez pas dans les issues/PR.

## Linux / macOS / Windows — même procédure

1. Ouvrez **https://www.deezer.com** dans votre navigateur (Chrome, Firefox, Edge…).
2. Connectez-vous avec votre compte.
3. Ouvrez les **outils développeur** :
   - Chrome/Edge : `Ctrl+Shift+I` (Windows/Linux) ou `Cmd+Alt+I` (macOS)
   - Firefox : `Ctrl+Shift+I` / `Cmd+Alt+I`
4. Allez dans l'onglet **Application** (Chrome/Edge) → **Cookies** → `www.deezer.com`
   (Firefox : **Stockage** → **Cookies**).
5. Trouvez le cookie nommé `arl`, copiez sa **Valeur** (longue chaîne hexadécimale, ~192 caractères).
6. Dans Breezer : barre latérale → *Connexion* → collez l'ARL → **Se connecter**.

Breezer valide ensuite l'ARL auprès de Deezer et récupère votre jeton API automatiquement.

## Dépannage

| Problème | Correctif |
|---|---|
| `Connexion refusée` | L'ARL est expiré ou a été régénéré — recopiez-le depuis le navigateur. |
| La page de connexion ne s'ouvre pas | Cliquez manuellement sur *Ouvrir le navigateur*, ou copiez l'URL Deezer dans votre navigateur. |
| Vous êtes déconnecté(e) | Les ARL tournent ; recollez-en un frais. Nous gardons le jeton en cache pour la session. |