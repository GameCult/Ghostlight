#!/usr/bin/env python3
"""Emit a TeX newspaper with independent house-style and issue-flow seeds."""
from __future__ import annotations

import argparse, hashlib, html, os, random, re, shutil, subprocess, tempfile
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
    paper: str; ink: str; fleuron: str; rules: str

@dataclass(frozen=True)
class Flow:
    seed: int; template: str; inside_layout: str; page_one_secondary: int
    lead_cut: float; inside_cut: float

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

def choose_style(seed: int) -> Style:
    r=random.Random(seed)
    return Style(seed,r.choice((9.6,9.8,10.0)),r.choice((10.7,10.9,11.1)),r.choice((.34,.38,.42)),
        r.choice((7.7,7.9,8.1)),r.choice((1.03,1.05,1.07)),r.choice((8.,9.5,11.)),
        r.choice((0.,.24,.36)),r.choice((36,39,42)),r.choice((25,27,29)),
        r.choice(("Georgia","Times New Roman")),"Times New Roman",
        r.choice(("F2E5C7","F5E9CE","EFE1C2")),r.choice(("17120D","21170F","1A1510")),
        r.choice(("diamond","cross","leaf")),r.choice(("double","heavy-thin","triple")))

def choose_flow(seed: int, e: Edition, cuts: dict[int, Path]) -> Flow:
    r=random.Random(seed); remaining=list(range(1,len(e.articles)))
    without_cuts=[index for index in remaining if index not in cuts]
    secondary=r.choice(without_cuts or remaining) if remaining else -1
    template=r.choice(("display-plate","display-band"))
    lead_cut=r.choice((.90,.97)) if template=="display-plate" else r.choice((.78,.82,.86))
    inside_layout=r.choice(("cut-first","cut-midstory"))
    inside_cut=r.choice((.62,.70,.78)) if inside_layout=="cut-first" else r.choice((.44,.50,.56))
    return Flow(seed,template,inside_layout,secondary,lead_cut,inside_cut)

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

def story_body(a: Article) -> str:
    return "\n\n".join(inline(p) for p in a.paragraphs)

def story_header(a: Article) -> str:
    sec=(r"{\sectionface\fontsize{7.2}{8}\selectfont\MakeUppercase{"+esc(a.section)+r"}}\par\smallskip " if a.section else "")
    head=r"\storyhead{"+esc(a.headline)+"}\n"
    return (r"\Needspace{18\baselineskip}\noindent\begin{minipage}{\linewidth}\hrule height .45pt\smallskip "+sec+head+
        r"\deck{"+esc(a.deck)+"}\n"+r"\byline{"+esc(a.author)+r"}\end{minipage}\par\smallskip")

def article(a: Article) -> str:
    return story_header(a)+story_body(a)

def rules(kind: str) -> str:
    return {"double":r"\hrule height 1.1pt\vspace{2pt}\hrule height .35pt","heavy-thin":r"\hrule height 1.7pt\vspace{1pt}\hrule height .25pt","triple":r"\hrule height .3pt\vspace{1pt}\hrule height 1.1pt\vspace{1pt}\hrule height .3pt"}[kind]

def ornament(kind: str) -> str:
    center={"diamond":r"\rotatebox{45}{\rule{5pt}{5pt}}","cross":r"\raisebox{-1pt}{\rule{1pt}{7pt}}\hspace{-4pt}\rule{7pt}{1pt}","leaf":r"\rotatebox{35}{\rule{3pt}{7pt}}\hspace{-1pt}\rotatebox{-35}{\rule{3pt}{7pt}}"}[kind]
    return r"\rule{18pt}{.35pt}\hspace{5pt}"+center+r"\hspace{5pt}\rule{18pt}{.35pt}"

def tex_path(p: Path) -> str: return p.resolve().as_posix()

def press_cut(path: Path, width: float) -> str:
    return (r"\begin{center}\begin{tikzpicture}\begin{scope}[blend mode=multiply]"
        r"\node[inner sep=0] {\includegraphics[width="+f"{width:.2f}"+r"\linewidth]{"+tex_path(path)+r"}};"
        r"\end{scope}\end{tikzpicture}\end{center}")

def build(e: Edition, s: Style, f: Flow, cuts: dict[int, Path], source: Path) -> str:
    lead=e.articles[0]
    remaining=list(enumerate(e.articles[1:],1))
    rail=next((pair for pair in remaining if pair[0]==f.page_one_secondary),None)
    paired=[pair for pair in remaining if pair != rail]
    leadcut=press_cut(cuts[0],f.lead_cut) if 0 in cuts else ""
    rail_text=(r"\par\medskip "+article(rail[1])) if rail else ""
    pre=rf"""\documentclass{{article}}
\usepackage[paperwidth={s.width}in,paperheight={s.height}in,margin={s.margin}in]{{geometry}}
\usepackage{{fontspec,graphicx,xcolor,microtype,multicol,ragged2e,fancyhdr,tikz,needspace}}
\setmainfont{{{s.text}}}\newfontfamily\displayface{{{s.display}}}\newfontfamily\sectionface{{{s.display}}}
\definecolor{{paper}}{{HTML}}{{{s.paper}}}\definecolor{{ink}}{{HTML}}{{{s.ink}}}\pagecolor{{paper}}\color{{ink}}
\setlength{{\columnsep}}{{{s.gutter}pt}}\setlength{{\columnseprule}}{{{s.rule}pt}}\setlength{{\parindent}}{{1em}}\setlength{{\parskip}}{{0pt}}\setlength{{\emergencystretch}}{{1em}}
\pagestyle{{fancy}}\fancyhf{{}}\renewcommand{{\headrulewidth}}{{0pt}}\fancyfoot[C]{{\displayface\fontsize{{6.5}}{{7}}\selectfont {esc(e.masthead.upper())} · HOUSE {s.seed} · FLOW {f.seed} · \thepage}}
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
\leadhead{{{esc(lead.headline)}}}\smallskip
{{\displayface\itshape\fontsize{{9.2}}{{10.5}}\selectfont\centering {esc(lead.deck)}\par}}
{{\displayface\bfseries\fontsize{{7}}{{8}}\selectfont\centering BY {esc(lead.author.upper())}\par}}\smallskip
{leadcut}\smallskip
\begin{{multicols}}{{4}}{story_body(lead)}
{rail_text}
\end{{multicols}}
\vfill\begin{{center}}\fleuron\end{{center}}
"""
    pages=[pre]; page=2
    while paired:
        group,paired=paired[:2],paired[2:]
        first_index,first=group[0]
        first_cut=press_cut(cuts[first_index],f.inside_cut) if first_index in cuts else ""
        first_header=story_header(first)
        flowing=story_body(first)
        for _,a in group[1:]: flowing+=r"\par\medskip "+article(a)
        if f.inside_layout=="cut-midstory" and first_cut:
            opening=inline(first.paragraphs[0]); remainder="\n\n".join(inline(p) for p in first.paragraphs[1:])
            for _,a in group[1:]: remainder+=r"\par\medskip "+article(a)
            inside=rf"""{first_header}\begin{{multicols}}{{4}}{opening}\end{{multicols}}{first_cut}\begin{{multicols}}{{4}}{remainder}\end{{multicols}}"""
        else:
            inside=rf"""{first_header}{first_cut}\begin{{multicols}}{{4}}{flowing}\end{{multicols}}"""
        pages.append(rf"""\clearpage\thispagestyle{{fancy}}{{\displayface\fontsize{{7}}{{8}}\selectfont {esc(e.edition.upper())}\hfill PAGE {page}\par}}\smallskip\pressrules\smallskip
{{\displayface\bfseries\fontsize{{18}}{{19}}\selectfont\centering {esc(e.masthead)}\par}}\smallskip\pressrules\medskip
{inside}
\vfill\begin{{center}}\fleuron\end{{center}}"""); page+=1
    return "\n".join(pages)+"\n\\end{document}\n"

def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def verify_copy(e: Edition, rendered: str) -> None:
    haystack=rendered.casefold(); missing=[]
    for index,a in enumerate(e.articles):
        fields=[("headline",esc(a.headline)),("deck",esc(a.deck)),("author",esc(a.author))]
        fields += [(f"paragraph-{n}",inline(p)) for n,p in enumerate(a.paragraphs,1)]
        missing += [(index,label) for label,value in fields if value.casefold() not in haystack]
    if missing: raise ValueError(f"press projection omitted frozen copy: {missing}")

def manifest(path: Path, src: Path, out: Path, tex: Path, cuts: dict[int, Path], style: Style, flow: Flow, engine: Path | None):
    tool=Path(__file__).resolve()
    rows=['schema = "ghostlight.seeded_tex_press.v2"',f'source = "{src.resolve().as_posix()}"',f'source_sha256 = "{digest(src)}"',f'output = "{out.resolve().as_posix()}"',f'tex = "{tex.resolve().as_posix()}"',f'tex_sha256 = "{digest(tex)}"',f'tool = "{tool.as_posix()}"',f'tool_sha256 = "{digest(tool)}"',f"style_seed = {style.seed}",f"flow_seed = {flow.seed}"]
    if engine:
        rows += [f'engine = "{engine.resolve().as_posix()}"',f'engine_sha256 = "{digest(engine)}"']
    if out.is_file(): rows.append(f'output_sha256 = "{digest(out)}"')
    rows += ["","[style]"]
    for k,v in asdict(style).items():
        if k!="seed": rows.append(f'{k} = "{v}"' if isinstance(v,str) else f"{k} = {v}")
    rows += ["","[flow]"]
    for k,v in asdict(flow).items():
        if k!="seed": rows.append(f'{k} = "{v}"' if isinstance(v,str) else f"{k} = {v}")
    rows += ["","[[woodcuts]]"] if cuts else []
    for position,(article_index,c) in enumerate(sorted(cuts.items())):
        if position: rows.append("[[woodcuts]]")
        rows += [f'article_index = {article_index}',f'path = "{c.resolve().as_posix()}"',f'sha256 = "{digest(c)}"']
    path.parent.mkdir(parents=True,exist_ok=True); path.write_text("\n".join(rows)+"\n",encoding="utf-8")

def compile(engine: Path, tex: Path, out: Path):
    out.parent.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ghostlight-tex-") as td:
        env=os.environ.copy(); env["SOURCE_DATE_EPOCH"]="0"; env["FORCE_SOURCE_DATE"]="1"
        p=subprocess.run([str(engine),"-interaction=nonstopmode","-halt-on-error",f"-output-directory={td}",str(tex.resolve())],capture_output=True,text=True,env=env)
        built=Path(td)/(tex.stem+".pdf")
        if p.returncode or not built.exists(): raise RuntimeError((p.stdout+p.stderr)[-6000:])
        shutil.copy2(built,out)

def main():
    ap=argparse.ArgumentParser(description=__doc__); ap.add_argument("source",type=Path); ap.add_argument("output",type=Path); ap.add_argument("--style-seed",type=int,required=True); ap.add_argument("--flow-seed",type=int,required=True); ap.add_argument("--woodcut",action="append",default=[],metavar="ARTICLE_INDEX=PATH"); ap.add_argument("--tex-output",type=Path); ap.add_argument("--manifest",type=Path); ap.add_argument("--engine",type=Path); ap.add_argument("--no-compile",action="store_true"); a=ap.parse_args()
    src=a.source.resolve(); out=a.output.resolve(); cuts={}
    for spec in a.woodcut:
        try: raw_index,raw_path=spec.split("=",1); article_index=int(raw_index)
        except ValueError as exc: raise ValueError("--woodcut requires ARTICLE_INDEX=PATH") from exc
        if article_index in cuts: raise ValueError(f"duplicate woodcut for article {article_index}")
        cuts[article_index]=Path(raw_path).resolve()
    for p in [src,*cuts.values()]:
        if not p.is_file(): raise FileNotFoundError(p)
    tx=(a.tex_output or Path("output/tex")/(out.stem+".tex")).resolve(); mf=(a.manifest or tx.with_suffix(".manifest.toml")).resolve(); ed=parse(src); st=choose_style(a.style_seed); fl=choose_flow(a.flow_seed,ed,cuts)
    if any(index < 0 or index >= len(ed.articles) for index in cuts): raise ValueError("woodcut article index is outside the edition")
    rendered=build(ed,st,fl,cuts,src); verify_copy(ed,rendered)
    tx.parent.mkdir(parents=True,exist_ok=True); tx.write_text(rendered,encoding="utf-8")
    eng=None
    if not a.no_compile:
        eng=a.engine or Path(shutil.which("lualatex") or "")
        if not eng.is_file(): raise FileNotFoundError("LuaLaTeX not found; pass --engine")
        compile(eng,tx,out)
    manifest(mf,src,out,tx,cuts,st,fl,eng)
    print(f"typeset={out} tex={tx} manifest={mf} style_seed={st.seed} flow_seed={fl.seed} articles={len(ed.articles)} pages={1+(max(0,len(ed.articles)-2)+1)//2}")

if __name__=="__main__": main()
