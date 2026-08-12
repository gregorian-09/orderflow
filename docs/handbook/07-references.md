# References and Source Links

This page lists external references used to shape the documentation structure and concept explanations.

## Documentation Standards and Structure

- ISO/IEC 26514 overview (user documentation requirements):  
  https://www.iso.org/standard/43073.html
- IEEE adoption page for ISO/IEC 26514:  
  https://ieeexplore.ieee.org/document/5712775
- Diataxis framework (official):  
  https://diataxis.fr/
- Diataxis quick start:  
  https://diataxis.fr/start-here/
- Read the Docs guidance on documentation structure (recommends Diataxis):  
  https://docs.readthedocs.com/platform/stable/explanation/documentation-structure.html

## Footprint / Volumetric Chart Construction References

- NinjaTrader Order Flow Volumetric Bars (official help):  
  https://ninjatrader.com/support/helpguides/nt8/order_flow_volumetric_bars.htm
- NinjaScript volumetric API references:  
  https://ninjatrader.com/support/helpguides/nt8/order_flow_volumetric_bars2.htm
- Sierra Chart Numbers Bars documentation:  
  https://www.sierrachart.com/index.php?page=doc/NumbersBars.php

## Market Microstructure and Execution Context

- Introductory market microstructure notes (Bath PDF):  
  https://people.bath.ac.uk/mnsak/Microstructure.pdf
- SEC Investor Bulletin on order types and execution behavior:  
  https://www.investor.gov/introduction-investing/general-resources/news-alerts/alerts-bulletins/investor-bulletins-14
- CME glossary (general derivatives terms):  
  https://www.cmegroup.com/education/glossary

## FIX Protocol References

- FIX Trading Community standards package index:
  https://fixtrading.org/standards/
- FIX Trading Community FIX 4.2 specification package:
  https://fixtrading.org/packages/fix-4-2-specification-with-errata-20010501/
- FIX Trading Community FIX 4.4 specification package:
  https://fixtrading.org/packages/fix-4-4-specification-with-20030618-errata/
- FIX Latest unified repository package:
  https://fixtrading.org/packages/fix-latest-unified-repository/
- FIX Latest Orchestra repository package:
  https://fixtrading.org/packages/fix-latest-orchestra-repository/
- FIX Trading Community automated trading and testing/certification guidance:
  https://fixtrading.org/guidelines/automated-trading/
- FIXimate `MsgType(35)` message semantics, including session recovery and
  order/execution message roles:
  https://fiximate.fixtrading.org/en/FIX.Latest/tag35.html
- OnixS FIX 4.2 checksum calculation reference:
  https://www.onixs.biz/fix-dictionary/4.2/app_b.html

## Rust Concurrency References

- Rust standard-library MPSC channel module, including bounded `sync_channel`,
  disconnection, and nonblocking `try_send` semantics:
  https://doc.rust-lang.org/std/sync/mpsc/
- Rust standard-library `sync_channel` FIFO and capacity contract:
  https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html
- Linux kernel network timestamping documentation, including userspace,
  kernel-software, and raw-hardware timestamp sources:
  https://docs.kernel.org/networking/timestamping.html

## Risk and Disclosure References

- CFTC Rule 4.41 (eCFR, official):  
  https://www.ecfr.gov/current/title-17/chapter-I/part-4/subpart-D/section-4.41
- NFA Compliance Rule 2-30 (risk disclosure suitability framework):  
  https://www.nfa.futures.org/rulebooksql/rules.aspx?Section=9&RuleID=9004

## Practitioner Ecosystem References (Context)

- Orderflows (Michael Valtos) official site:  
  https://www.orderflows.com/
- Orderflows setup taxonomy page:  
  https://www.orderflows.com/setups.html

## Notes on Source Use

- API and architecture sections are sourced from the repository code.
- Platform links (NinjaTrader/Sierra) are used for terminology and chart-construction conventions.
- Market behavior sections are educational and do not imply strategy performance.
