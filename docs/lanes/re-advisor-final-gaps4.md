| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-advisor-final-gaps4 | `C:\Users\tlori\FreeCity4\.claude\worktrees\re-advisor-final-gaps4` (branch `re-advisor-final-gaps4`) | in progress | answer AD12-AD15 |

## Handoff

- Goal: answer docs/specs/questions/advisors.md AD12-AD15 (advice text GUIDs, DoLink, advisor QI, head anims) into docs/specs/advisors.md 6.9/12.2/12.3/1.1/3.2/4.4, move to answered, run tools/questions.py index.
- Done: lane file; RE of all four done (not yet written): AD12 GUID = cSLString key instance (+0x30/+0x50, key type 0x2026960B group 0x6A231EA4, instance from sscanf "text@0x%08x","text@%08x","text@%u","text@%i" in cADVString::init 0x4178F0); rebuild 0x409300 re-reads advices[i].message/title via Lua, URL-decodes, parses key, cSLString::Load, on fail "***_NOT_IN_TEXT_DATABASE_*** "+text (cp 0x4E4), ConvertTokens (detokenizer HTML-clean on), 0x200 debug "=text@%08X= %s =adv@%08X="; no "news" field. AD13 DoLink 0x407ED0: flag 8 set or not active -> true; else PushContextAdvice(this), DoBuffer(ToChar, strlen, same), PopContextAdvice, true. AD14 QI 0x40D430: 0x452294AA +0x10, 0xCA09F544 +8, 0xE4FDA3D4 +0xC, 1 -> this, else false (out untouched); CityPlanning 0x411810 adds 0x2A5F877D; cSC4MySim 0x424B00 0x0A1DA42F (+0xB8), 0x8BC01FFE, zeroes out first. AD15 load_heads 0x40FA10 (global advisor_heads, advisor_type==GetID, advisor_gender==1 -> list true else false), Set 0x40D3E0, ID 0x40F3A0 (advisor_heads[list[cur]][k] ToSint32).
- Next: write into advisors.md 2.2/4.5/6.9/12.2/12.3/17, move AD12-15 to answered, questions.py index.


## Coordinator quota pause (2026-10-22)
Provider weekly limit; resets 18:00 Europe/London. This round stopped. Existing work is preserved as WIP, not ready or verified. Resume only after the provider limit clears and fresh slots/check --resume pass; the round still counts.
