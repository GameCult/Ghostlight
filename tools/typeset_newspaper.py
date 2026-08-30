#!/usr/bin/env python3
"""Emit and compile a reproducible seed-varied TeX newspaper."""
from __future__ import annotations

import argparse, hashlib, html, random, re, shutil, subprocess, tempfile
from dataclasses import asdict, dataclass
from pathlib import Path

@dataclass(frozen=True)
class Article:
    section: str | None; headline: str; deck: str; author: str; paragraphs: tuple[str, ...]

@dataclass(frozen=True)
class Edition:
    masthead: str; edition: str; articles: tuple[Article, ...]

@dataclass(frozen=True)
class Style:
    seed: int; width: float; height: float; margin: float; body: float; leading: float
    gutter: float; rule: float; masthead: int; lead: int; display: str; text: str
    paper: str; ink: str; fleuron: str; lead_cut: float; inside_cut: float; rules: str

def plain(s: str) -> str:
    s=s.strip()
    if s.startswith("**") and s.endswith("**"): s=s[2:-2]
    elif s[:1] in "*_" and s[-1:]==s[:1]: s=s[1:-1]
    return html.unescape(s)

def parse(path: Path) -> Edition:
    lines=path.read_text(encoding="utf-8").splitlines(); mast=""; ed=""; sec=None; out=[]; i=0
    while i < len(lines):
        s=lines[i].strip()
        if s.startswith("# ") and not mast: mast=plain(s[2:])
        elif not ed and s.startswith("*") and s.endswith("*"): ed=plain(s)
        elif s.startswith("### "): sec=plain(s[4:])
        elif s.startswith("## "):
            head=plain(s[3:]); i+=1
            while not lines[i].strip(): i+=1
            deck=plain(lines[i]); i+=1
            while not lines[i].strip(): i+=1
            author=plain(lines[i]); i+=1; paras=[]; acc=[]
            while i < len(lines):
                t=lines[i].strip()
                if t=="---" or t.startswith(("## ","### ")):
                    if acc: paras.append(" ".join(acc)); acc=[]
                    break
                if t: acc.append(t)
                elif acc: paras.append(" ".join(acc)); acc=[]
                i+=1
            if acc: paras.append(" ".join(acc))
            out.append(Article(sec,head,deck,author,tuple(paras))); sec=None; continue
        i+=1
    if not mast or not ed or not out: raise ValueError("source lacks masthead, edition, or articles")
    return Edition(mast,ed,tuple(out))

def choose(seed: int) -> Style:
    r=random.Random(seed)
    return Style(seed,r.choice((9.8,10.0,10.2)),r.choice((13.7,14.0,14.3)),r.choice((.38,.42,.46)),
        r.choice((8.6,8.8,9.0)),r.choice((1.04,1.07,1.10)),r.choice((9.,10.5,12.)),
        r.choice((0.,.24,.36)),r.choice((36,39,42)),r.choice((25,27,29)),
        r.choice(("Georgia","Times New Roman")),"Times New Roman",
        r.choice(("F2E5C7","F5E9CE","EFE1C2")),r.choice(("17120D","21170F","1A1510")),
        r.choice(("diamond","cross","leaf")),r.choice((.78,.82,.86)),r.choice((.44,.50,.54)),
        r.choice(("double","heavy-thin","triple")))

def esc(s: str) -> str:
    m={"\\":r"\textbackslash{}","&":r"\&","%":r"\%","$":r"\$","#":r"\#","_":r"\_","{":r"\{","}":r"\}","~":r"\textasciitilde{}","^":r"\textasciicircum{}"}
    return "".join(m.get(c,c) for c in html.unescape(s))

def inline(s: str) -> str:
    out=[]
    for p in re.split(r"(\*\*.*?\*\*|\*.*?\*)",html.unescape(s)):
        if p.startswith("**") and p.endswith("**"): out.append(r"\textbf{"+esc(p[2:-2])+"}")
        elif p.startswith("*") and p.endswith("*"): out.append(r"\textit{"+esc(p[1:-1])+"}")
        else: out.append(esc(p))
    return "".join(out)

def article(a: Article, lead=False) -> str:
    if lead:
        return "\n\n".join(inline(p) for p in a.paragraphs)
    sec=(r"{\sectionface\fontsize{7.2}{8}\selectfont\MakeUppercase{"+esc(a.section)+r"}}\par\smallskip " if a.section else "")
    head=r"\storyhead{"+esc(a.headline)+"}\n"
    return sec+head+r"\deck{"+esc(a.deck)+"}\n"+r"\byline{"+esc(a.author)+"}\n\n"+"\n\n".join(inline(p) for p in a.paragraphs)

def rules(kind: str) -> str:
    return {"double":r"\hrule height 1.1pt\vspace{2pt}\hrule height .35pt","heavy-thin":r"\hrule height 1.7pt\vspace{1pt}\hrule height .25pt","triple":r"\hrule height .3pt\vspace{1pt}\hrule height 1.1pt\vspace{1pt}\hrule height .3pt"}[kind]

def ornament(kind: str) -> str:
    center={"diamond":r"\rotatebox{45}{\rule{5pt}{5pt}}","cross":r"\raisebox{-1pt}{\rule{1pt}{7pt}}\hspace{-4pt}\rule{7pt}{1pt}","leaf":r"\rotatebox{35}{\rule{3pt}{7pt}}\hspace{-1pt}\rotatebox{-35}{\rule{3pt}{7pt}}"}[kind]
    return r"\rule{18pt}{.35pt}\hspace{5pt}"+center+r"\hspace{5pt}\rule{18pt}{.35pt}"

def tex_path(p: Path) -> str: return p.resolve().as_posix()

def build(e: Edition, s: Style, cuts: list[Path], source: Path) -> str:
    leadcut=""
    if cuts:
        leadcut=r"\noindent\includegraphics[width="+f"{s.lead_cut:.2f}"+r"\textwidth]{"+tex_path(cuts[0])+"}"
        leadcut+=r"\par"
    pre=rf"""\documentclass{{article}}
\usepackage[paperwidth={s.width}in,paperheight={s.height}in,margin={s.margin}in]{{geometry}}
\usepackage{{fontspec,graphicx,xcolor,microtype,multicol,ragged2e,fancyhdr}}
\setmainfont{{{s.text}}}\newfontfamily\displayface{{{s.display}}}\newfontfamily\sectionface{{{s.display}}}
\definecolor{{paper}}{{HTML}}{{{s.paper}}}\definecolor{{ink}}{{HTML}}{{{s.ink}}}\pagecolor{{paper}}\color{{ink}}
\setlength{{\columnsep}}{{{s.gutter}pt}}\setlength{{\columnseprule}}{{{s.rule}pt}}\setlength{{\parindent}}{{1em}}\setlength{{\parskip}}{{0pt}}\setlength{{\emergencystretch}}{{1em}}
\pagestyle{{fancy}}\fancyhf{{}}\renewcommand{{\headrulewidth}}{{0pt}}\fancyfoot[C]{{\displayface\fontsize{{6.5}}{{7}}\selectfont {esc(e.masthead.upper())} · SEED {s.seed} · \thepage}}
\newcommand{{\pressrules}}{{{rules(s.rules)}}}\newcommand{{\fleuron}}{{{ornament(s.fleuron)}}}
\newcommand{{\masthead}}[1]{{{{\displayface\bfseries\fontsize{{{s.masthead}}}{{{s.masthead+2}}}\selectfont\centering\MakeUppercase{{#1}}\par}}}}
\newcommand{{\leadhead}}[1]{{{{\displayface\bfseries\fontsize{{{s.lead}}}{{{s.lead+1.5}}}\selectfont\centering #1\par}}}}
\newcommand{{\storyhead}}[1]{{{{\displayface\bfseries\fontsize{{14}}{{15}}\selectfont\RaggedRight #1\par\smallskip}}}}
\newcommand{{\deck}}[1]{{{{\displayface\itshape\fontsize{{8}}{{9.2}}\selectfont\RaggedRight #1\par\smallskip}}}}
\newcommand{{\byline}}[1]{{{{\displayface\bfseries\fontsize{{6.8}}{{7.5}}\selectfont\MakeUppercase{{By #1}}\par\smallskip}}}}
\AtBeginDocument{{\fontsize{{{s.body}}}{{{s.body*s.leading:.2f}}}\selectfont\justifying}}
\begin{{document}}\thispagestyle{{fancy}}
{{\displayface\fontsize{{7}}{{8}}\selectfont {esc(e.edition.upper())}\hfill SOURCE: {esc(source.name)}\par}}\smallskip\pressrules\smallskip
\masthead{{{esc(e.masthead)}}}\smallskip\pressrules\medskip
\leadhead{{{esc(e.articles[0].headline)}}}\smallskip
{{\displayface\itshape\fontsize{{9.2}}{{10.5}}\selectfont\centering {esc(e.articles[0].deck)}\par}}
{{\displayface\bfseries\fontsize{{7}}{{8}}\selectfont\centering BY {esc(e.articles[0].author.upper())}\par}}\smallskip
{leadcut}\smallskip
\begin{{multicols}}{{3}}{article(e.articles[0],True)}\end{{multicols}}\vfill\begin{{center}}\fleuron\end{{center}}
"""
    pages=[pre]; rest=list(e.articles[1:]); page=2; ci=1
    while rest:
        group,rest=rest[:3],rest[3:]
        bottomcut=""
        if ci<len(cuts):
            bottomcut=r"\vfill\begin{center}\includegraphics[width="+f"{s.inside_cut:.2f}"+r"\textwidth]{"+tex_path(cuts[ci])+r"}\end{center}\smallskip"; ci+=1
        cols=[r"\begin{minipage}[t]{\linewidth}"+article(a)+r"\end{minipage}" for a in group]+[""]*3
        pages.append(rf"""\clearpage\thispagestyle{{fancy}}{{\displayface\fontsize{{7}}{{8}}\selectfont {esc(e.edition.upper())}\hfill PAGE {page}\par}}\smallskip\pressrules\smallskip
{{\displayface\bfseries\fontsize{{18}}{{19}}\selectfont\centering {esc(e.masthead)}\par}}\smallskip\pressrules\medskip
\noindent
\begin{{minipage}}[t]{{\dimexpr(\textwidth-2\columnsep)/3\relax}}{cols[0]}\end{{minipage}}\hspace{{\columnsep}}%
\begin{{minipage}}[t]{{\dimexpr(\textwidth-2\columnsep)/3\relax}}{cols[1]}\end{{minipage}}\hspace{{\columnsep}}%
\begin{{minipage}}[t]{{\dimexpr(\textwidth-2\columnsep)/3\relax}}{cols[2]}\end{{minipage}}{bottomcut}\begin{{center}}\fleuron\end{{center}}"""); page+=1
    return "\n".join(pages)+"\n\\end{document}\n"

def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def manifest(path: Path, src: Path, out: Path, tex: Path, cuts: list[Path], style: Style, engine: Path | None):
    tool=Path(__file__).resolve()
    rows=['schema = "ghostlight.seeded_tex_press.v1"',f'source = "{src.resolve().as_posix()}"',f'source_sha256 = "{digest(src)}"',f'output = "{out.resolve().as_posix()}"',f'tex = "{tex.resolve().as_posix()}"',f'tex_sha256 = "{digest(tex)}"',f'tool = "{tool.as_posix()}"',f'tool_sha256 = "{digest(tool)}"',f"seed = {style.seed}"]
    if engine:
        rows += [f'engine = "{engine.resolve().as_posix()}"',f'engine_sha256 = "{digest(engine)}"']
    if out.is_file(): rows.append(f'output_sha256 = "{digest(out)}"')
    rows += ["","[style]"]
    for k,v in asdict(style).items():
        if k!="seed": rows.append(f'{k} = "{v}"' if isinstance(v,str) else f"{k} = {v}")
    rows += ["","[[woodcuts]]"] if cuts else []
    for index,c in enumerate(cuts):
        if index: rows.append("[[woodcuts]]")
        rows += [f'path = "{c.resolve().as_posix()}"',f'sha256 = "{digest(c)}"']
    path.parent.mkdir(parents=True,exist_ok=True); path.write_text("\n".join(rows)+"\n",encoding="utf-8")

def compile(engine: Path, tex: Path, out: Path):
    out.parent.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ghostlight-tex-") as td:
        p=subprocess.run([str(engine),"-interaction=nonstopmode","-halt-on-error",f"-output-directory={td}",str(tex.resolve())],capture_output=True,text=True)
        built=Path(td)/(tex.stem+".pdf")
        if p.returncode or not built.exists(): raise RuntimeError((p.stdout+p.stderr)[-6000:])
        shutil.copy2(built,out)

def main():
    ap=argparse.ArgumentParser(description=__doc__); ap.add_argument("source",type=Path); ap.add_argument("output",type=Path); ap.add_argument("--seed",type=int,required=True); ap.add_argument("--woodcut",type=Path,action="append",default=[]); ap.add_argument("--tex-output",type=Path); ap.add_argument("--manifest",type=Path); ap.add_argument("--engine",type=Path); ap.add_argument("--no-compile",action="store_true"); a=ap.parse_args()
    src=a.source.resolve(); out=a.output.resolve(); cuts=[p.resolve() for p in a.woodcut]
    for p in [src,*cuts]:
        if not p.is_file(): raise FileNotFoundError(p)
    tx=(a.tex_output or Path("output/tex")/(out.stem+".tex")).resolve(); mf=(a.manifest or tx.with_suffix(".manifest.toml")).resolve(); st=choose(a.seed); ed=parse(src)
    tx.parent.mkdir(parents=True,exist_ok=True); tx.write_text(build(ed,st,cuts,src),encoding="utf-8")
    eng=None
    if not a.no_compile:
        eng=a.engine or Path(shutil.which("lualatex") or "")
        if not eng.is_file(): raise FileNotFoundError("LuaLaTeX not found; pass --engine")
        compile(eng,tx,out)
    manifest(mf,src,out,tx,cuts,st,eng)
    print(f"typeset={out} tex={tx} manifest={mf} seed={st.seed} articles={len(ed.articles)} pages={1+(len(ed.articles)-1+2)//3}")

if __name__=="__main__": main()
