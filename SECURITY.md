# Security Policy

## Supported versions

cameo is pre-1.0; security fixes land on the latest published `0.x` release.

## Reporting a vulnerability

Please **do not** open a public issue for security problems. Instead, use
GitHub's [private vulnerability reporting](https://github.com/justin13888/cameo/security/advisories/new)
("Report a vulnerability" under the repository's **Security** tab). We aim to
acknowledge reports within a few days and to coordinate a fix and disclosure
timeline with you.

## Handling API tokens

cameo talks to third-party APIs (TMDB, AniList) and takes credentials from you:

- **Never commit tokens.** Provide them at runtime — the examples read
  `TMDB_API_TOKEN` from the environment or a local `.env` (see
  [`.env.example`](.env.example)); `.env` is git-ignored.
- Tokens are held only in `TmdbConfig`/`AniListConfig` and sent over HTTPS.
  cameo does not log credential values.
- Prefer a TMDB **v4 Read Access Token** (bearer) over a v3 API key where you
  can; both are supported (`TmdbAuth::V4Bearer` / `TmdbAuth::V3ApiKey`).
- Treat the on-disk cache (`~/.cache/cameo/cache.db` by default) as containing
  API response data; it does **not** store your credentials.
