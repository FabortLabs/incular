
## Method

- Haiku 5.5 agents (effort medium) audited each unit against a condensed checklist (`CHECKLIST.md`). Units were capped at about 1,100 lines, which kept each agent under the 100K-token pricing tier.
- `verify.py` checks mechanically that each finding's evidence exists at the cited line. This catches misread or invented citations.
- Opus read the code for:
  - every high-severity finding;
  - a random sample of 20 medium/low findings (seed 7).
- The sample had 0 wrong findings and one overstated `public_api` flag. A second sample of 10 unreviewed high-severity allocation findings (seed 11) also looked correct; 2 of them were read in full against the code. Unverified medium/low findings and unreviewed high allocation findings are therefore treated as likely valid.
- Verdict meanings:
  - `systemic:<name>`: the finding belongs to one workspace-wide fix (see Systemic issues).
  - `needs-design`: the fix changes behavior or structure, or needs a benchmark first.
