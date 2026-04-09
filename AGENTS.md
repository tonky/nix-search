it's april 2026, use moderl libraries, frameworks and industry standards.

don't start non-trivial implementation if it's not planned in detail. investigate and write down plan and stages into 'impl/<year-month-feature_name>' before starting work. actively encourage user to do this.

write down inital feature description into DESCRIPTION.md in feature dir. investigate, iterate and confirm with user on high level implementation plan. write down agreed high level plan into PLAN.md.

after plan is written - confirm its correctness witth user, before breaking work down into stages, each being independently implemented and verifiable, that could be implemented in 8 hours of work, with enough details and context.
use substages with separate file per substage, if individual stage scope is too big.
write down those stages as separate files.

before asking user to review planned stages - launch a subagent to investigate and verify that stages cover plan, order is correct and check for overall improvements or reorders.

before starting implementing a stage - investigate it and clarify, if needed, any questions, inconsistencies or suggested improvements for this stage.

after stage implementation is clear - start working on it autonomously until stage is done.

after stage is completed -  run review in subagent for conformance to planned work, assess it, fix any discrepancies and notify user of stage completion.

put improvement ideas, postponed work and follow ups into FOLLOW_UP.tml in impl feature with minimum context and description that it can be reviewed and potentially implemented later.

minimal follow-up TOML example:

[[item]]
title = "Investigate browser E2E coverage"

keep WORKLOG.md per stage of your progress and important decisions/tradeoffs made, and remember to keep updating them.

after each stage is complete with subagent review:
 - check feature FOLLOW_UP.toml for any work that can improve or extend implemented stage or other aspects of what was already implemented before. defer ones that should be implemented later or unclear.





 -----------------
 don't use global /tmp for temporary files, use repo local 'tmp/' dir to avoid problems with permissions.

 aim for good test coverage, prefer high level and end to end tests that validate behavior.

 use 'flox' for any local dependency that you need. do not use brew on macos.

 use 'act' to run and verify github actions locally, where possible.