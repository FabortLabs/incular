import { Link } from "@tanstack/react-router";
import { HomeLayout, useHomeLayout } from "fumadocs-ui/layouts/home";
import {
  ArrowDown,
  ArrowRight,
  ArrowUpRight,
  Braces,
  Check,
  Command,
  Copy,
  Layers3,
  Menu,
  Monitor,
  Terminal,
  X,
  Zap,
} from "lucide-react";
import { useEffect, useState } from "react";
import { baseOptions } from "@/lib/layout.shared";
import { gitConfig } from "@/lib/shared";

const githubUrl = `https://github.com/${gitConfig.user}/${gitConfig.repo}`;
const exampleCommand = "cargo run -p incular --example hello";

function Github({ size = 18 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
    >
      <path d="M12 2C6.48 2 2 6.48 2 12c0 4.42 2.87 8.17 6.84 9.49.5.09.68-.22.68-.48 0-.24-.01-1.03-.01-1.87-2.78.6-3.37-1.18-3.37-1.18-.45-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.61.07-.61 1.01.07 1.54 1.04 1.54 1.04.9 1.53 2.36 1.09 2.94.83.09-.65.35-1.09.64-1.34-2.22-.25-4.55-1.11-4.55-4.94 0-1.09.39-1.99 1.03-2.69-.1-.25-.45-1.27.1-2.65 0 0 .84-.27 2.75 1.03A9.58 9.58 0 0 1 12 6.82c.85 0 1.71.11 2.51.34 1.91-1.3 2.75-1.03 2.75-1.03.55 1.38.2 2.4.1 2.65.64.7 1.03 1.6 1.03 2.69 0 3.84-2.34 4.68-4.57 4.93.36.31.68.92.68 1.85 0 1.34-.01 2.42-.01 2.75 0 .27.18.58.69.48A10.001 10.001 0 0 0 22 12c0-5.52-4.48-10-10-10Z" />
    </svg>
  );
}

const demos = [
  {
    id: "hello",
    label: "Hello, native",
    nativeExample: "hello",
    nativeSource: "crates/incular/examples/hello.rs",
    guide: "introduction/quick-start",
    description: "Create a native application with a centered text widget.",
    code: `use incular::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new(|_cx| {
        Container::builder()
            .padding(EdgeInsets::all(24.0))
            .alignment(Alignment::CENTER)
            .child(Text::new("Hello, Incular!"))
            .build()
            .into()
    })?;

    incular::run(app)?;
    Ok(())
}`,
  },
  {
    id: "state",
    label: "Reactive state",
    nativeExample: "counter",
    nativeSource: "examples/counter/main.rs",
    guide: "api/state/signal",
    description:
      "Read a signal during build to subscribe your widgets to state changes.",
    code: `use incular::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let count = Signal::new(0u32);
    let app = Application::new(move |_cx| {
        let value = count.get();
        Container::builder()
            .padding(EdgeInsets::all(24.0))
            .alignment(Alignment::CENTER)
            .child(Text::new(format!("Count: {value}")))
            .build()
            .into()
    })?;

    incular::run(app)?;
    Ok(())
}`,
  },
] as const;

const features = [
  {
    icon: Layers3,
    title: "Small widgets. Big ideas.",
    description:
      "Compose a little. Create a lot. Build rich interfaces from focused, reusable widgets that fit together naturally.",
    link: "Explore the widgets",
    path: "api/layout-widgets/widget",
  },
  {
    icon: Zap,
    title: "State that stays in sync.",
    description:
      "Connect your UI to reactive signals. When state changes, Incular rebuilds the parts of your interface that depend on it.",
    link: "Meet reactive state",
    path: "api/state/signal",
  },
  {
    icon: Braces,
    title: "Rust, all the way down.",
    description:
      "Your layout, your logic, your language. Keep your application in Rust, with native windows and a WGPU rendering pipeline.",
    link: "See how it works",
    path: "introduction/what-is-incular",
  },
];

function DocsLink({
  path = "introduction/quick-start",
  className,
  children,
  onClick,
}: {
  path?: string;
  className?: string;
  children: React.ReactNode;
  onClick?: () => void;
}) {
  return (
    <Link
      to="/docs/$"
      params={{ _splat: path }}
      className={className}
      onClick={onClick}
    >
      {children}
    </Link>
  );
}

function LandingHeader() {
  const { slots } = useHomeLayout();
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    if (!menuOpen) return;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setMenuOpen(false);
    };
    document.addEventListener("keydown", closeOnEscape);
    return () => document.removeEventListener("keydown", closeOnEscape);
  }, [menuOpen]);

  return (
    <header className="landing-header">
      <a href="#landing-content" className="landing-skip-link">
        Skip to content
      </a>
      <div className="landing-container landing-header-inner">
        <slots.navTitle className="landing-brand" />
        <nav aria-label="Main navigation" className="landing-desktop-nav">
          <a href="#why-incular">Why Incular</a>
          <a href="#examples">Examples</a>
          <DocsLink path="">Documentation</DocsLink>
        </nav>
        <div className="landing-header-actions">
          {slots.searchTrigger && (
            <slots.searchTrigger.sm className="landing-search-trigger" />
          )}
          {slots.themeSwitch && <slots.themeSwitch />}
          <a
            href={githubUrl}
            target="_blank"
            rel="noreferrer"
            aria-label="Incular on GitHub (opens in a new tab)"
            className="landing-github-icon"
          >
            <Github size={19} />
          </a>
          <DocsLink className="landing-nav-cta">
            Get started <ArrowUpRight size={15} />
          </DocsLink>
          <button
            type="button"
            className="landing-menu-trigger"
            aria-label={menuOpen ? "Close navigation" : "Open navigation"}
            aria-expanded={menuOpen}
            aria-controls="landing-mobile-nav"
            onClick={() => setMenuOpen((value) => !value)}
          >
            {menuOpen ? <X size={21} /> : <Menu size={21} />}
          </button>
        </div>
      </div>
      <nav
        id="landing-mobile-nav"
        aria-label="Mobile navigation"
        className="landing-mobile-nav"
        hidden={!menuOpen}
      >
        <Link to="/" hash="why-incular" onClick={() => setMenuOpen(false)}>
          Why Incular <ArrowDown size={16} />
        </Link>
        <Link to="/" hash="examples" onClick={() => setMenuOpen(false)}>
          Examples <ArrowDown size={16} />
        </Link>
        <DocsLink path="" onClick={() => setMenuOpen(false)}>
          Documentation <ArrowUpRight size={16} />
        </DocsLink>
        <DocsLink onClick={() => setMenuOpen(false)}>
          Get started <ArrowRight size={16} />
        </DocsLink>
      </nav>
    </header>
  );
}

function CopyButton({ text, label }: { text: string; label: string }) {
  const [status, setStatus] = useState<"idle" | "copied" | "error">("idle");

  useEffect(() => {
    if (status === "idle") return;
    const timeout = window.setTimeout(() => setStatus("idle"), 2500);
    return () => window.clearTimeout(timeout);
  }, [status]);

  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
      setStatus("copied");
    } catch {
      setStatus("error");
    }
  }

  return (
    <span className="landing-copy-control">
      <button
        type="button"
        className="landing-copy-button"
        aria-label={status === "copied" ? "Copied!" : label}
        title={status === "copied" ? "Copied!" : label}
        onClick={copy}
      >
        {status === "copied" ? <Check size={15} /> : <Copy size={15} />}
      </button>
      <span role="status" className="landing-copy-status">
        {status === "copied" && "Copied!"}
        {status === "error" && "Select the text to copy manually."}
      </span>
    </span>
  );
}

// A projected wireframe torus, drawn as vectors so it stays crisp at any size.
const torusLines = Array.from({ length: 64 }, (_, ring) => {
  const angle = (ring / 64) * Math.PI * 2;
  let depth = 0;
  const points = Array.from({ length: 81 }, (_, step) => {
    const cross = (step / 80) * Math.PI * 2;
    const radius = 154 + 67 * Math.cos(cross);
    const x = radius * Math.cos(angle);
    const y = radius * Math.sin(angle);
    const z = 67 * Math.sin(cross);
    const tiltedY = y * Math.cos(0.95) - z * Math.sin(0.95);
    const tiltedZ = y * Math.sin(0.95) + z * Math.cos(0.95);
    const turnedX = x * Math.cos(0.35) + tiltedZ * Math.sin(0.35);
    const turnedZ = -x * Math.sin(0.35) + tiltedZ * Math.cos(0.35);
    depth += turnedZ;
    const finalX = turnedX * Math.cos(-0.5) - tiltedY * Math.sin(-0.5);
    const finalY = turnedX * Math.sin(-0.5) + tiltedY * Math.cos(-0.5);
    return `${step === 0 ? "M" : "L"}${(300 + finalX * 1.23).toFixed(2)},${(290 + finalY * 1.23).toFixed(2)}`;
  });
  return {
    id: `ring-${ring}`,
    path: `${points.join(" ")}Z`,
    depth: depth / 81,
  };
}).sort((a, b) => a.depth - b.depth);

function OrbitalSculpture() {
  return (
    <svg
      viewBox="0 0 600 580"
      fill="none"
      aria-hidden="true"
      className="landing-sculpture"
    >
      <defs>
        <linearGradient id="landing-orbit" x1="60" y1="90" x2="520" y2="440">
          <stop stopColor="#b6a0ff" />
          <stop offset="0.4" stopColor="#9563ff" />
          <stop offset="1" stopColor="#6300df" />
        </linearGradient>
      </defs>
      {torusLines.map((line) => (
        <path
          key={line.id}
          d={line.path}
          stroke="url(#landing-orbit)"
          strokeWidth={line.depth > 0 ? 3.2 : 2}
          opacity={line.depth > 0 ? 0.95 : 0.45}
        />
      ))}
    </svg>
  );
}

function HeroScene() {
  return (
    <div className="landing-hero-scene" aria-hidden="true">
      <OrbitalSculpture />
    </div>
  );
}

function HighlightedCode({ code }: { code: string }) {
  const tokens =
    /"(?:[^"\\]|\\.)*"|\/\/.*|\b(?:use|fn|let|move|dyn)\b|\b(?:Application|Container|EdgeInsets|Alignment|Signal|Text|Box|Result|Error|Ok)\b|\b\d+(?:\.\d+)?(?:u32)?\b/g;
  const lines = code.split("\n").map((line, lineNumber) => {
    const segments: React.ReactNode[] = [];
    let cursor = 0;
    for (const match of line.matchAll(tokens)) {
      if (match.index > cursor) segments.push(line.slice(cursor, match.index));
      const token = match[0];
      const kind = token.startsWith('"')
        ? "string"
        : token.startsWith("//")
          ? "comment"
          : /^(use|fn|let|move|dyn)$/.test(token)
            ? "keyword"
            : /^\d/.test(token)
              ? "number"
              : "type";
      segments.push(
        <span key={`token-${match.index}`} className={`landing-code-${kind}`}>
          {token}
        </span>,
      );
      cursor = match.index + token.length;
    }
    segments.push(line.slice(cursor));
    return { id: `line-${lineNumber + 1}`, number: lineNumber + 1, segments };
  });

  return (
    <section
      className="landing-code-region"
      // biome-ignore lint/a11y/noNoninteractiveTabindex: Keyboard users need to scroll the code region horizontally.
      tabIndex={0}
      aria-label="Rust example source code"
    >
      <pre className="landing-code">
        <code>
          {lines.map((line) => (
            <span className="landing-code-line" key={line.id}>
              <span className="landing-line-number" aria-hidden="true">
                {line.number}
              </span>
              <span>
                {line.segments}
                {"\n"}
              </span>
            </span>
          ))}
        </code>
      </pre>
    </section>
  );
}

function NativeExamples() {
  const [selected, setSelected] =
    useState<(typeof demos)[number]["id"]>("hello");
  const demo = demos.find((item) => item.id === selected) ?? demos[0];

  return (
    <section id="examples" className="landing-examples landing-container">
      <div className="landing-section-heading">
        <div>
          <h2>
            From Rust to <em>something real.</em>
          </h2>
        </div>
        <DocsLink path="api/examples" className="landing-text-link">
          Explore the examples <ArrowUpRight size={17} />
        </DocsLink>
      </div>
      <div className="landing-workbench">
        <div className="landing-workbench-toolbar">
          <fieldset className="landing-demo-selector">
            <legend className="sr-only">Choose a code example</legend>
            {demos.map((item) => (
              <button
                type="button"
                key={item.id}
                aria-pressed={selected === item.id}
                onClick={() => setSelected(item.id)}
              >
                {item.id === "hello" ? (
                  <Terminal size={14} />
                ) : (
                  <Zap size={14} />
                )}
                {item.label}
              </button>
            ))}
          </fieldset>
        </div>
        <div className="landing-workbench-body">
          <div className="landing-editor">
            <div className="landing-editor-bar">
              <span>
                <Braces size={14} /> main.rs
              </span>
              <CopyButton text={demo.code} label="Copy Rust example" />
            </div>
            <HighlightedCode code={demo.code} />
          </div>
          <div className="landing-example-guide">
            <h3>Try it on your desktop.</h3>
            <p>{demo.description}</p>
            <p>
              These snippets introduce the API. From a{" "}
              <DocsLink path="introduction/installation">
                local checkout
              </DocsLink>
              , run the complete {demo.nativeExample} example in a native
              window:
            </p>
            <div className="landing-command">
              <code>cargo run -p incular --example {demo.nativeExample}</code>
              <CopyButton
                text={`cargo run -p incular --example ${demo.nativeExample}`}
                label="Copy native example command"
              />
            </div>
            <div className="landing-example-links">
              <DocsLink path={demo.guide} className="landing-text-link">
                Read the guide <ArrowUpRight size={16} />
              </DocsLink>
              <a
                href={`${githubUrl}/blob/${gitConfig.branch}/${demo.nativeSource}`}
                className="landing-text-link"
                target="_blank"
                rel="noreferrer"
              >
                View native example source <ArrowUpRight size={16} />
              </a>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

export function LandingPage() {
  return (
    <HomeLayout
      {...baseOptions()}
      slots={{ header: LandingHeader }}
      className="incular-home"
    >
      <div id="landing-content">
        <section
          className="landing-hero landing-container"
          aria-labelledby="landing-title"
        >
          <div className="landing-hero-copy">
            <h1 id="landing-title">
              Native apps.
              <br />
              Unbound <em>ideas.</em>
            </h1>
            <p className="landing-hero-description">
              A native Rust UI framework for the way you think. Compose your
              interface. Connect your state.
              <br className="landing-desktop-break" /> Bring something new to
              life.
            </p>
            <div className="landing-hero-buttons">
              <DocsLink className="landing-button landing-button-primary">
                Start building <ArrowUpRight size={19} />
              </DocsLink>
              <DocsLink
                path="api/overview"
                className="landing-button landing-button-secondary"
              >
                Explore the API <ArrowRight size={18} />
              </DocsLink>
            </div>
            <div className="landing-command">
              <Terminal size={15} aria-hidden="true" />
              <code>{exampleCommand}</code>
              <CopyButton text={exampleCommand} label="Copy example command" />
            </div>
            <p className="landing-command-note">
              From a{" "}
              <DocsLink path="introduction/installation">
                local checkout
              </DocsLink>{" "}
              to your first native window.
            </p>
          </div>
          <HeroScene />
        </section>

        <div className="landing-platform-strip landing-container">
          <p>
            ONE CODEBASE.
            <br />
            <strong>RIGHT AT HOME, EVERYWHERE.</strong>
          </p>
          <ul
            className="landing-platforms"
            aria-label="Supported desktop platforms"
          >
            <li>
              <Monitor size={21} /> Windows
            </li>
            <li>
              <Command size={21} /> macOS
            </li>
            <li>
              <Terminal size={21} /> Linux
            </li>
          </ul>
          <a
            href="#why-incular"
            className="landing-scroll-link"
            aria-label="Discover why Incular"
          >
            <ArrowDown size={19} />
          </a>
        </div>

        <section id="why-incular" className="landing-why landing-container">
          <div className="landing-section-heading landing-why-heading">
            <div>
              <h2>
                Thoughtfully simple.
                <br />
                <em>Powerfully yours.</em>
              </h2>
            </div>
            <p>
              Your ideas already have enough moving parts.
              <br className="landing-desktop-break" /> Your UI framework should
              bring them together.
            </p>
          </div>
          <div className="landing-features">
            {features.map((feature) => (
              <article className="landing-feature" key={feature.path}>
                <div className="landing-feature-top">
                  <feature.icon size={25} strokeWidth={1.5} />
                </div>
                <h3>{feature.title}</h3>
                <p>{feature.description}</p>
                <DocsLink path={feature.path} className="landing-text-link">
                  {feature.link}
                  <ArrowUpRight size={16} />
                </DocsLink>
              </article>
            ))}
          </div>
        </section>

        <NativeExamples />

        <section
          className="landing-closing landing-container"
          aria-labelledby="landing-closing-title"
        >
          <div className="landing-closing-art" aria-hidden="true">
            <span />
            <span />
            <span />
            <span />
            <span />
          </div>
          <div className="landing-closing-copy">
            <h2 id="landing-closing-title">
              Give your ideas
              <br />a <em>native home.</em>
            </h2>
            <p>Start small. Make it yours. See where it takes you.</p>
            <div className="landing-closing-buttons">
              <DocsLink className="landing-button landing-button-light">
                Get started <ArrowUpRight size={18} />
              </DocsLink>
              <a
                href={githubUrl}
                target="_blank"
                rel="noreferrer"
                className="landing-closing-github"
              >
                <Github size={18} /> Find us on GitHub{" "}
                <ArrowUpRight size={15} />
              </a>
            </div>
          </div>
        </section>

        <footer className="landing-footer landing-container">
          <div className="landing-footer-brand">
            <img src={`${import.meta.env.BASE_URL}incular.svg`} alt="" />
            <span>Incular</span>
            <span className="landing-footer-tagline">
              Rust at heart. Native by nature.
            </span>
          </div>
          <div className="landing-footer-links">
            <DocsLink path="">Docs</DocsLink>
            <a href={githubUrl} target="_blank" rel="noreferrer">
              GitHub <ArrowUpRight size={12} />
            </a>
            <a
              href={`${githubUrl}/blob/${gitConfig.branch}/LICENSE`}
              target="_blank"
              rel="noreferrer"
            >
              Apache 2.0 <ArrowUpRight size={12} />
            </a>
          </div>
        </footer>
      </div>
    </HomeLayout>
  );
}
