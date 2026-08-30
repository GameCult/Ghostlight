#!/usr/bin/env python3
"""Typeset a compact Markdown newspaper front page as a press-ready PDF."""

from __future__ import annotations

import argparse
import html
import math
import random
import re
from dataclasses import dataclass
from pathlib import Path
from xml.sax.saxutils import escape

from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER, TA_JUSTIFY, TA_LEFT
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.units import inch
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen.canvas import Canvas
from reportlab.platypus import Frame, Paragraph


INK = colors.HexColor("#17120d")
PAPER = colors.HexColor("#f2e6ca")
HAIRLINE = colors.HexColor("#4b3b2b")
FOLIO = (10 * inch, 12.2 * inch)


@dataclass(frozen=True)
class Article:
    section: str | None
    headline: str
    deck: str
    author: str
    paragraphs: tuple[str, ...]


@dataclass(frozen=True)
class Edition:
    masthead: str
    edition: str
    articles: tuple[Article, ...]


def register_fonts() -> None:
    font_root = Path("C:/Windows/Fonts")
    faces = {
        "LedgerSerif": "times.ttf",
        "LedgerSerifBold": "timesbd.ttf",
        "LedgerSerifItalic": "timesi.ttf",
        "LedgerDisplay": "georgia.ttf",
        "LedgerDisplayBold": "georgiab.ttf",
        "LedgerDisplayItalic": "georgiai.ttf",
    }
    for family, filename in faces.items():
        path = font_root / filename
        if not path.exists():
            raise FileNotFoundError(f"Required font is unavailable: {path}")
        pdfmetrics.registerFont(TTFont(family, str(path)))
    pdfmetrics.registerFontFamily(
        "LedgerSerif",
        normal="LedgerSerif",
        bold="LedgerSerifBold",
        italic="LedgerSerifItalic",
        boldItalic="LedgerSerifBold",
    )
    pdfmetrics.registerFontFamily(
        "LedgerDisplay",
        normal="LedgerDisplay",
        bold="LedgerDisplayBold",
        italic="LedgerDisplayItalic",
        boldItalic="LedgerDisplayBold",
    )


def _plain_markdown(text: str) -> str:
    text = text.strip()
    if len(text) >= 4 and text.startswith("**") and text.endswith("**"):
        text = text[2:-2]
    elif len(text) >= 2 and text[:1] == text[-1:] and text[:1] in {"*", "_"}:
        text = text[1:-1]
    return html.unescape(text)


def _inline_markup(text: str) -> str:
    """Render only the tiny inline-Markdown subset used by the frozen copy."""
    source = html.unescape(text)
    parts = re.split(r"(\*\*.*?\*\*|\*.*?\*)", source)
    rendered: list[str] = []
    for part in parts:
        if part.startswith("**") and part.endswith("**"):
            rendered.append(f"<b>{escape(part[2:-2])}</b>")
        elif part.startswith("*") and part.endswith("*"):
            rendered.append(f"<i>{escape(part[1:-1])}</i>")
        else:
            rendered.append(escape(part))
    return "".join(rendered)


def parse_edition(path: Path) -> Edition:
    lines = path.read_text(encoding="utf-8").splitlines()
    masthead = ""
    edition = ""
    articles: list[Article] = []
    section: str | None = None
    index = 0

    while index < len(lines):
        line = lines[index].strip()
        if line.startswith("# ") and not masthead:
            masthead = _plain_markdown(line[2:])
        elif not edition and line.startswith("*") and line.endswith("*"):
            edition = _plain_markdown(line)
        elif line.startswith("### "):
            section = _plain_markdown(line[4:])
        elif line.startswith("## "):
            headline = _plain_markdown(line[3:])
            index += 1
            while index < len(lines) and not lines[index].strip():
                index += 1
            deck = _plain_markdown(lines[index])
            index += 1
            while index < len(lines) and not lines[index].strip():
                index += 1
            author = _plain_markdown(lines[index])
            index += 1
            paragraphs: list[str] = []
            paragraph_lines: list[str] = []
            while index < len(lines):
                current = lines[index].strip()
                if current == "---" or current.startswith("### ") or current.startswith("## "):
                    if paragraph_lines:
                        paragraphs.append(" ".join(paragraph_lines))
                        paragraph_lines.clear()
                    break
                if current:
                    paragraph_lines.append(current)
                elif paragraph_lines:
                    paragraphs.append(" ".join(paragraph_lines))
                    paragraph_lines.clear()
                index += 1
            if paragraph_lines:
                paragraphs.append(" ".join(paragraph_lines))
            articles.append(Article(section, headline, deck, author, tuple(paragraphs)))
            section = None
            continue
        index += 1

    if not masthead or not edition or len(articles) != 3:
        raise ValueError(
            f"Expected masthead, edition label, and three articles; got "
            f"masthead={bool(masthead)}, edition={bool(edition)}, articles={len(articles)}"
        )
    return Edition(masthead, edition, tuple(articles))


def _styles(body_size: float) -> dict[str, ParagraphStyle]:
    return {
        "body": ParagraphStyle(
            "body",
            fontName="LedgerSerif",
            fontSize=body_size,
            leading=body_size * 1.18,
            alignment=TA_JUSTIFY,
            textColor=INK,
            spaceAfter=body_size * 0.72,
            allowWidows=0,
            allowOrphans=0,
        ),
        "section": ParagraphStyle(
            "section",
            fontName="LedgerDisplayBold",
            fontSize=7.3,
            leading=8.4,
            alignment=TA_LEFT,
            textColor=INK,
            spaceAfter=3.5,
            borderPadding=(2.2, 0, 1.5, 0),
            borderWidth=0,
            borderColor=HAIRLINE,
        ),
        "side_head": ParagraphStyle(
            "side_head",
            fontName="LedgerDisplayBold",
            fontSize=15.5,
            leading=16.3,
            alignment=TA_LEFT,
            textColor=INK,
            spaceAfter=5.2,
        ),
        "side_deck": ParagraphStyle(
            "side_deck",
            fontName="LedgerDisplayItalic",
            fontSize=8.2,
            leading=10.0,
            alignment=TA_LEFT,
            textColor=INK,
            spaceAfter=5.0,
        ),
        "byline": ParagraphStyle(
            "byline",
            fontName="LedgerDisplayBold",
            fontSize=7.4,
            leading=8.5,
            alignment=TA_LEFT,
            textColor=INK,
            spaceAfter=5.5,
        ),
    }


def _paper_texture(canvas: Canvas, width: float, height: float) -> None:
    canvas.setFillColor(PAPER)
    canvas.rect(0, 0, width, height, fill=1, stroke=0)
    random.seed(6043)
    canvas.saveState()
    try:
        canvas.setStrokeAlpha(0.10)
    except AttributeError:
        pass
    for _ in range(150):
        shade = random.choice((colors.HexColor("#8d7354"), colors.HexColor("#fff7e4")))
        canvas.setStrokeColor(shade)
        canvas.setLineWidth(random.choice((0.15, 0.22, 0.3)))
        x = random.uniform(0.2 * inch, width - 0.2 * inch)
        y = random.uniform(0.2 * inch, height - 0.2 * inch)
        length = random.uniform(7, 32)
        canvas.line(x, y, min(width - 0.2 * inch, x + length), y + random.uniform(-0.5, 0.5))
    canvas.restoreState()


def _pressmark(canvas: Canvas, x: float, y: float, scale: float = 1.0) -> None:
    """A small one-color canopy mark, drawn like a worn printer's ornament."""
    canvas.saveState()
    canvas.translate(x, y)
    canvas.scale(scale, scale)
    canvas.setStrokeColor(INK)
    canvas.setLineCap(1)
    canvas.setLineJoin(1)
    canvas.setLineWidth(1.0)
    canvas.circle(0, 0, 20, fill=0, stroke=1)
    canvas.line(0, -14, 0, 10)
    for dx, dy, ex, ey in (
        (0, 7, -13, 15),
        (0, 6, 13, 15),
        (0, 2, -16, 7),
        (0, 2, 16, 7),
        (-4, -3, -13, -8),
        (4, -3, 13, -8),
    ):
        canvas.line(dx, dy, ex, ey)
    for offset in (-3, 0, 3):
        canvas.line(-9, -14 + offset * 0.25, 9, -14 + offset * 0.25)
    canvas.restoreState()


def _draw_rule(canvas: Canvas, x1: float, y: float, x2: float, width: float = 0.7) -> None:
    canvas.saveState()
    canvas.setStrokeColor(INK)
    canvas.setLineWidth(width)
    canvas.line(x1, y, x2, y + 0.15)
    canvas.restoreState()


def _root_rail_cut(canvas: Canvas, x: float, y: float, width: float, height: float) -> None:
    """Wordless relief-print ornament: roots crossing a breached pair of rails."""
    canvas.saveState()
    canvas.setStrokeColor(INK)
    canvas.setFillColor(INK)
    canvas.setLineCap(1)
    canvas.setLineJoin(1)
    canvas.setLineWidth(0.75)
    canvas.rect(x, y, width, height, fill=0, stroke=1)
    pad = 8
    for offset in (0.34, 0.66):
        rail_x = x + width * offset
        canvas.setLineWidth(2.0)
        canvas.line(rail_x, y + pad, rail_x, y + height - pad)
        canvas.setLineWidth(0.45)
        canvas.line(rail_x - 3, y + pad, rail_x - 3, y + height - pad)
    for rung_y in range(int(y + 13), int(y + height - 8), 12):
        canvas.setLineWidth(0.65)
        canvas.line(x + width * 0.27, rung_y, x + width * 0.73, rung_y + 1.2)

    roots = (
        [(0.08, 0.08), (0.20, 0.25), (0.38, 0.30), (0.55, 0.52), (0.89, 0.88)],
        [(0.02, 0.68), (0.24, 0.56), (0.46, 0.60), (0.70, 0.38), (0.98, 0.30)],
        [(0.18, 0.98), (0.29, 0.77), (0.55, 0.71), (0.78, 0.57), (0.96, 0.10)],
    )
    canvas.setLineWidth(3.1)
    for points in roots:
        path = canvas.beginPath()
        first_x, first_y = points[0]
        path.moveTo(x + first_x * width, y + first_y * height)
        for point_x, point_y in points[1:]:
            path.lineTo(x + point_x * width, y + point_y * height)
        canvas.drawPath(path, fill=0, stroke=1)
    canvas.setLineWidth(0.38)
    for index in range(24):
        hatch_x = x + pad + (index * 17) % max(18, int(width - 2 * pad))
        hatch_y = y + pad + (index * 29) % max(18, int(height - 2 * pad))
        canvas.line(hatch_x, hatch_y, hatch_x + 11, hatch_y + 5)
    canvas.restoreState()


def _gauge_cut(canvas: Canvas, x: float, y: float, width: float, height: float) -> None:
    """Wordless relief-print ornament: a calibrated counterflow dial."""
    canvas.saveState()
    canvas.setStrokeColor(INK)
    canvas.setFillColor(INK)
    canvas.setLineCap(1)
    canvas.setLineJoin(1)
    canvas.setLineWidth(0.75)
    canvas.rect(x, y, width, height, fill=0, stroke=1)
    radius = min(width * 0.30, height * 0.38)
    cx = x + width / 2
    cy = y + height * 0.50
    canvas.setLineWidth(1.5)
    canvas.circle(cx, cy, radius, fill=0, stroke=1)
    canvas.circle(cx, cy, radius * 0.82, fill=0, stroke=1)
    for step in range(13):
        angle = 3.1415926535 * (0.12 + 0.063 * step)
        inner = radius * (0.70 if step % 3 else 0.62)
        outer = radius * 0.84
        canvas.line(
            cx + inner * math.cos(angle),
            cy + inner * math.sin(angle),
            cx + outer * math.cos(angle),
            cy + outer * math.sin(angle),
        )
    canvas.setLineWidth(2.4)
    canvas.line(cx, cy, cx + radius * 0.45, cy + radius * 0.48)
    canvas.circle(cx, cy, 3.2, fill=1, stroke=0)
    canvas.setLineWidth(0.45)
    for offset in (-8, -4, 0, 4, 8):
        canvas.line(x + 10, y + 10 + offset * 0.2, x + width * 0.25, cy + offset)
        canvas.line(x + width - 10, y + 10 - offset * 0.2, x + width * 0.75, cy - offset)
    canvas.restoreState()


def _build_article(article: Article, styles: dict[str, ParagraphStyle], include_header: bool) -> list:
    story: list = []
    if include_header:
        if article.section:
            story.append(Paragraph(escape(article.section.upper()), styles["section"]))
        story.append(Paragraph(escape(article.headline), styles["side_head"]))
        story.append(Paragraph(escape(article.deck), styles["side_deck"]))
        story.append(Paragraph(f"BY {escape(article.author.upper())}", styles["byline"]))
    else:
        story.append(Paragraph(f"BY {escape(article.author.upper())}", styles["byline"]))
    for paragraph in article.paragraphs:
        story.append(Paragraph(_inline_markup(paragraph), styles["body"]))
    return story


def _draw_page(canvas: Canvas, edition: Edition, body_size: float) -> bool:
    width, height = FOLIO
    _paper_texture(canvas, width, height)
    margin_x = 0.48 * inch
    usable_width = width - 2 * margin_x

    # Printer's strapline and masthead.
    canvas.setFillColor(INK)
    canvas.setFont("LedgerDisplay", 7.2)
    canvas.drawString(margin_x, height - 0.34 * inch, edition.edition.upper())
    _draw_rule(canvas, margin_x, height - 0.43 * inch, width - margin_x, 0.55)

    masthead_y = height - 0.91 * inch
    _pressmark(canvas, margin_x + 22, masthead_y + 3, 0.83)
    _pressmark(canvas, width - margin_x - 22, masthead_y + 3, 0.83)
    canvas.setFont("LedgerDisplayBold", 32)
    canvas.drawCentredString(width / 2, masthead_y - 7, edition.masthead.upper())
    _draw_rule(canvas, margin_x, height - 1.23 * inch, width - margin_x, 1.65)
    _draw_rule(canvas, margin_x, height - 1.29 * inch, width - margin_x, 0.42)

    # Lead banner.
    lead = edition.articles[0]
    lead_head = ParagraphStyle(
        "lead_head",
        fontName="LedgerDisplayBold",
        fontSize=25,
        leading=26.2,
        alignment=TA_CENTER,
        textColor=INK,
    )
    lead_deck = ParagraphStyle(
        "lead_deck",
        fontName="LedgerDisplayItalic",
        fontSize=9.5,
        leading=11.8,
        alignment=TA_CENTER,
        textColor=INK,
    )
    headline = Paragraph(escape(lead.headline), lead_head)
    _, headline_h = headline.wrap(usable_width - 0.35 * inch, 0.8 * inch)
    headline_y = height - 1.45 * inch - headline_h
    headline.drawOn(canvas, margin_x + 0.175 * inch, headline_y)
    deck = Paragraph(escape(lead.deck), lead_deck)
    _, deck_h = deck.wrap(usable_width - 0.85 * inch, 0.5 * inch)
    deck_y = headline_y - 0.09 * inch - deck_h
    deck.drawOn(canvas, margin_x + 0.425 * inch, deck_y)
    column_top = deck_y - 0.16 * inch
    _draw_rule(canvas, margin_x, column_top, width - margin_x, 0.7)
    column_top -= 0.12 * inch

    gutter = 0.20 * inch
    column_width = (usable_width - 2 * gutter) / 3
    column_bottom = 0.42 * inch
    column_height = column_top - column_bottom
    styles = _styles(body_size)
    all_fit = True
    for index, article in enumerate(edition.articles):
        x = margin_x + index * (column_width + gutter)
        if index:
            divider_x = x - gutter / 2
            canvas.setStrokeColor(HAIRLINE)
            canvas.setLineWidth(0.35)
            canvas.line(divider_x, column_bottom, divider_x, column_top)
        frame = Frame(
            x,
            column_bottom,
            column_width,
            column_height,
            leftPadding=0,
            rightPadding=0,
            topPadding=0,
            bottomPadding=0,
            showBoundary=0,
        )
        story = _build_article(article, styles, include_header=index != 0)
        frame.addFromList(story, canvas)
        if story:
            all_fit = False

    if all_fit:
        _root_rail_cut(canvas, margin_x, column_bottom + 4, column_width, 2.45 * inch)
        _gauge_cut(
            canvas,
            margin_x + column_width + gutter,
            column_bottom + 4,
            column_width,
            1.25 * inch,
        )

    return all_fit


def render(edition: Edition, output: Path) -> float:
    output.parent.mkdir(parents=True, exist_ok=True)
    last_size = 0.0
    for body_size in (10.0, 9.7, 9.4, 9.1, 8.8, 8.5, 8.15):
        last_size = body_size
        canvas = Canvas(str(output), pagesize=FOLIO, pageCompression=1)
        canvas.setTitle(edition.masthead)
        canvas.setAuthor("The Canopy Ledger")
        fit = _draw_page(canvas, edition, body_size)
        canvas.showPage()
        canvas.save()
        if fit:
            return body_size
    raise RuntimeError(f"Copy did not fit at minimum body size {last_size}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="Markdown front page")
    parser.add_argument("output", type=Path, help="Output PDF")
    args = parser.parse_args()
    register_fonts()
    edition = parse_edition(args.source)
    body_size = render(edition, args.output)
    print(f"typeset={args.output} body_size={body_size:.2f} articles={len(edition.articles)}")


if __name__ == "__main__":
    main()
