# Parser fixtures

Small, attributed excerpts used by parser tests. They are not part of the
application binary and are never bundled or redistributed as data.

## `callersbox/`

- `search.html` — a trimmed search-results page from
  <https://www.ibiblio.org/contradance/thecallersbox/> (The Caller's Box, a
  contra dance database by Chris Page and Michael Dyck). The result table and
  the “your query matches” line keep the site's shape; navigation and the form
  are removed.
- `dance-13997.json` — the JSON record for *And Another Thing* by Chris Page,
  from `dance.php?id=13997&format=JSON`. The record is used only to test the
  permission, phrase, and beat-count conversion.

Dance records remain the work of their choreographers and of The Caller's Box
editors. The app fetches the live site on demand and caches responses locally;
these files exist so the tests never make a network request.
