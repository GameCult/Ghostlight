# Historical Newspaper Layout Grammar

This is a working press grammar, not a costume catalogue. It distills recurring
page structures from representative nineteenth-century newspapers into rules
the seeded TeX press can actually enforce.

## Reference set

- *Pall Mall Gazette*, first issue, 1865: a deliberately spacious evening
  review using two wide columns, double masthead rules, restrained italic
  headlines, and long uninterrupted reading measure.
  <https://academic.oup.com/res/article/77/330/336/8425236>
- *Evening Star* (Washington), 28 February 1880 and 17 June 1890: metropolitan
  sheets with a broad nameplate, six or more narrow continuous columns, dense
  vertical packing, and advertisements/notices inhabiting the same column grid
  as news.
  <https://www.loc.gov/resource/sn83045462/1880-02-28/ed-1/>
  <https://www.loc.gov/resource/sn83045462/1890-06-17/ed-1/>
- *The Sun* (New York), 30 May 1885: a uniform multi-column field with modest
  story heads and almost no unowned white space.
  <https://www.loc.gov/resource/sn83030272/1885-05-30/ed-1/>
- University of Illinois, *American Newspapers, 1800–1860*: physical and
  production context for five-to-eight-column sheets, full-height columns,
  front-page advertising, and the migration of timely news to inside pages.
  <https://www.library.illinois.edu/hpnl/tutorials/antebellum-newspapers-introduction/>
- Robinson, *Newspapers and Advertising*: nineteenth-century advertising moves
  from small text-heavy insertions intermingled on front and back pages toward
  larger display advertising and more consistent classified pages late in the
  century.
  <https://academic.oup.com/edinburgh-scholarship-online/book/42878/chapter-abstract/361009569>
- *New-York Tribune*, 16 April 1912: a page-wide disaster head, central map and
  photographic display spanning several columns, flanking report columns, and
  dense continuation below the illustration.
  <https://www.loc.gov/resource/sn83030214/1912-04-16/ed-1/>
- *Richmond Times-Dispatch*, 17 April 1912: a page-wide head and ship image
  followed by modular three-column packages, lists, subheads, and boxed matter.
  <https://www.lva.virginia.gov/events/exhibitions/titanic/newspaper_coverage.php>
- *St. Louis Globe-Democrat*, 17 April 1912: a large central illustrated package
  surrounded by independent column-width reports and subordinate display heads.
  <https://commons.wikimedia.org/wiki/File:St._Louis_Globe-Democrat_17_Apr_1912.png>
- *The Daily News* (St. John's), 14 August 1914: narrow continuous news columns
  coexist with multi-column war heads and boxed display advertisements occupying
  ordinary grid modules.
  <https://collections.mun.ca/digital/collection/dailynews/id/240059/>

Reference scans used for visual comparison live in disposable
`tmp/newspaper/references/`; the URLs above remain the durable provenance.

## Families

The press selects a family before it varies details.

### Evening review

- Two wide columns.
- Smaller physical sheet and lower information density.
- Long-form reading dominates scanning.
- Sparse display hierarchy; italics, rules, and small capitals do more work
  than large headlines.

### Metropolitan sheet

- Four to eight narrow columns depending on sheet size and date.
- The nameplate spans the page, while issue data, price, notices, or subscription
  terms may flank or bracket it.
- Stories, notices, and advertisements inhabit one shared vertical grid.
- Story heads are usually modest. Late-century sensational sheets may spend
  several columns on a display head or illustration, but the surrounding text
  closes around that expenditure.

The Canopy Ledger currently uses a compact four-column metropolitan sheet. Its
small accepted copy body does not earn a six-to-eight-column broadsheet.

### Mass-circulation display sheet, 1895–1914

- The underlying narrow-column grid survives, but a major event may spend
  several columns on one head, deck stack, map, photograph, or illustration.
- Display packages remain attached to their story. Copy resumes directly below
  them or rises in adjacent columns; the illustration does not become a detached
  gallery object.
- Secondary stories become more visibly modular through short rules, boxed
  lists, subheads, and column-spanning heads.
- A large central image may organize the entire page, with flanking text fields
  and subordinate modules closing around it.
- Display advertisements become visually distinct boxed modules while small ads
  and notices continue to occupy ordinary column measure.

The accepted Run 61 special issue uses this family. Its crisis head and cut earn
a page-wide display package; accepted story copy then returns immediately to the
four-column grid.

## Composition rules

1. **The page is a column economy.** Every occupied region begins and ends on
   column or half-column lines. Large vacant regions require an owned reason
   such as an absent paid advertisement; they are not a centering technique.
2. **Build fields, not isolated boxes.** A story field contains its head, deck,
   byline, cut, caption when present, and body measure. Adjacent fields share a
   top line or establish an intentional stepped hierarchy.
3. **Cuts spend story measure.** A narrative cut is assigned to one story and
   placed inside that story's contiguous field. It may span multiple columns,
   with the story flowing above, beside, or below it. It may not be deposited at
   a page edge after unrelated columns have ended.
4. **Ink blends with stock.** Monochrome cuts use multiply compositing (or its
   alpha-equivalent against a flat stock color) so white image paper disappears
   and black linework shares the page ink.
5. **Reading order remains typographic.** Column traversal, rules, heads, and
   continuation labels must make the next fragment unambiguous. Geometry may
   not reorder accepted copy.
6. **Density follows the edition.** Type size, page dimensions, column count,
   and cut expenditure are solved together from copy volume. A short edition
   becomes a smaller sheet or fewer columns; it does not become a broadsheet
   with a desert under it.
7. **Hierarchy is rationed.** One lead field may spend a multi-column head and
   large cut. Secondary heads stay within their field. Masthead display does not
   authorize every story to shout.
8. **Rules join related matter.** Column rules and horizontal rules articulate
   the grid. Ornaments close or divide small matter; they do not substitute for
   missing content.
9. **Advertisements are economic content.** The press may reserve typed ad
   slots by column width and depth. Only the economic layer may fill them with
   advertiser identity, goods, price, claims, campaign continuity, and payment.
   Unfilled slots collapse unless the edition explicitly records unsold space.
10. **House and issue variation are separate.** `style_seed` owns the durable
    publication body: sheet, grid, fonts, type scale, stock, ink, rules, gutter,
    and ornament vocabulary. `flow_seed` owns one issue's imposition: display
    template, story grouping, page-one closure, cut span and placement, page
    breaks, and continuation rhythm. Neither may detach art, rewrite copy,
    fabricate ads, or create barren pages.
11. **A house remains recognizable across flows.** Holding `style_seed` fixed
    must preserve typography, stock, ink, rules, sheet geometry, and base grid
    while materially different `flow_seed` values produce distinct plausible
    readings of the same admitted content. Holding `flow_seed` fixed while the
    style changes must preserve its story/cut topology.

## Current Run 61 imposition

- House style seed `1847`; issue flow seed `723`. The manifest records both as
  independent inputs under `ghostlight.seeded_tex_press.v2`.
- Page one: full-width nameplate and crisis head; a centered page-wide Boundary
  Rail cut; lead and secondary copy flowing continuously through four columns.
- Page two: a compact repeated nameplate and Sinkroot head; the centered gauge
  cut immediately below its owning head; Sinkroot and Dispatches copy flowing
  continuously through the same four-column grid.
- Secondary story headers are unbreakable column modules, so a section label or
  head cannot strand at the foot of the preceding column.
- Both cuts use PDF multiply blending so their white image stock disappears into
  the seeded paper stock. No image bank or detached bottom art reservation exists.
