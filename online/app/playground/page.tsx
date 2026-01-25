"use client";

import { Suspense, useEffect, useState, useRef, useCallback } from "react";
import { useSearchParams } from "next/navigation";
import Link from "next/link";

const docs = {
  quickstart: [
    { slug: "intro", title: "introduction" },
    { slug: "syntax", title: "syntax" },
    { slug: "types", title: "types" },
    { slug: "functions", title: "functions" },
    { slug: "structs", title: "structs" },
  ],
  developer: [
    { slug: "architecture", title: "architecture" },
    { slug: "parser", title: "parser" },
    { slug: "typechecker", title: "type checker" },
    { slug: "codegen", title: "code generation" },
    { slug: "gc", title: "garbage collection" },
  ],
};

interface CompilerExports {
  memory: WebAssembly.Memory;
  wasm_alloc: (len: number) => number;
  wasm_compile: (ptr: number, len: number) => number;
  wasm_result_ptr: () => number;
  wasm_result_len: () => number;
  wasm_error_ptr: () => number;
  wasm_error_len: () => number;
}

interface RuntimeModules {
  alloc: WebAssembly.Instance;
  dalloc: WebAssembly.Instance;
  shadow: WebAssembly.Instance;
}

export default function Playground() {
  return (
    <Suspense fallback={<div className="h-screen flex items-center justify-center">loading...</div>}>
      <PlaygroundContent />
    </Suspense>
  );
}

function PlaygroundContent() {
  const searchParams = useSearchParams();
  const docPath = searchParams.get("doc") || "quickstart/intro";
  const [category, slug] = docPath.split("/");

  const [docContent, setDocContent] = useState("");
  const [code, setCode] = useState(`struct Node {
    value: integer
}

fn main(): integer {
    let n: Node = new Node {
        value: 42,
    };
    return n.value;
}`);
  const [output, setOutput] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isRunning, setIsRunning] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<"docs" | "code">("docs");

  const compilerRef = useRef<WebAssembly.Instance | null>(null);
  const runtimeRef = useRef<RuntimeModules | null>(null);

  // Load WASM modules on mount
  useEffect(() => {
    async function loadWasm() {
      try {
        // Load compiler
        const compilerModule = await WebAssembly.instantiateStreaming(
          fetch("/wasm/compiler.wasm")
        );
        compilerRef.current = compilerModule.instance;

        // Load runtime modules
        const [allocModule, dallocModule] = await Promise.all([
          WebAssembly.instantiateStreaming(fetch("/wasm/alloc.wasm")),
          WebAssembly.instantiateStreaming(fetch("/wasm/dalloc.wasm")),
        ]);

        // Shadow needs imports from alloc and dalloc
        const shadowModule = await WebAssembly.instantiateStreaming(
          fetch("/wasm/shadow.wasm"),
          {
            alloc: allocModule.instance.exports,
            dalloc: dallocModule.instance.exports,
          }
        );

        runtimeRef.current = {
          alloc: allocModule.instance,
          dalloc: dallocModule.instance,
          shadow: shadowModule.instance,
        };

        setIsLoading(false);
        setOutput("// ready - click run to compile and execute");
      } catch (err) {
        setOutput(`// failed to load wasm modules:\n// ${err}`);
        setIsLoading(false);
      }
    }
    loadWasm();
  }, []);

  // Load docs
  useEffect(() => {
    fetch(`/docs/${docPath}.md`)
      .then((r) => (r.ok ? r.text() : "# not found\n\nthis doc doesn't exist yet."))
      .then((md) => setDocContent(md))
      .catch(() => setDocContent("# error\n\nfailed to load doc."));
  }, [docPath]);

  const handleRun = useCallback(async () => {
    if (!compilerRef.current || !runtimeRef.current) {
      setOutput("// wasm modules not loaded yet");
      return;
    }

    setIsRunning(true);
    setOutput("// compiling...");

    try {
      const compiler = compilerRef.current.exports as unknown as CompilerExports;
      const encoder = new TextEncoder();
      const sourceBytes = encoder.encode(code);

      // Allocate memory for source code
      const sourcePtr = compiler.wasm_alloc(sourceBytes.length);
      const memory = new Uint8Array(compiler.memory.buffer);
      memory.set(sourceBytes, sourcePtr);

      // Compile
      const success = compiler.wasm_compile(sourcePtr, sourceBytes.length);

      if (success === 0) {
        // Get error message
        const errorPtr = compiler.wasm_error_ptr();
        const errorLen = compiler.wasm_error_len();
        const errorBytes = new Uint8Array(compiler.memory.buffer, errorPtr, errorLen);
        const errorMsg = new TextDecoder().decode(errorBytes);
        setOutput(`// compile error:\n${errorMsg}`);
        setIsRunning(false);
        return;
      }

      // Get compiled WASM bytes
      const resultPtr = compiler.wasm_result_ptr();
      const resultLen = compiler.wasm_result_len();
      const wasmBytes = new Uint8Array(compiler.memory.buffer, resultPtr, resultLen).slice();

      setOutput("// running...");

      // Collect print output
      const printOutput: string[] = [];
      const runtime = runtimeRef.current;
      const dallocMemory = runtime.dalloc.exports.memory as WebAssembly.Memory;

      // Print function reads strings from dalloc memory
      const printFn = (ptr: number) => {
        const data = new Uint8Array(dallocMemory.buffer);
        // Length is stored at ptr - 4
        const length = new DataView(dallocMemory.buffer).getUint32(ptr - 4, true);

        // Each char is stored as 8 bytes (only first byte used)
        const chars: number[] = [];
        for (let i = 0; i < length; i++) {
          chars.push(data[ptr + i * 8]);
        }

        const str = String.fromCharCode(...chars);
        printOutput.push(str);
      };

      // Instantiate the compiled program with runtime imports
      const programModule = await WebAssembly.instantiate(wasmBytes, {
        env: { print: printFn },
        alloc: runtime.alloc.exports,
        dalloc: runtime.dalloc.exports,
        shadow: runtime.shadow.exports,
      });

      // Run main with (0, 0, 0) args
      const programExports = programModule.instance.exports as { main?: (a: number, b: bigint, c: number) => bigint };
      if (programExports.main) {
        const result = programExports.main(0, BigInt(0), 0);
        const outputText = printOutput.length > 0
          ? printOutput.join("\n") + `\n\nMain returned: ${result}`
          : `Main returned: ${result}`;
        setOutput(outputText);
      } else {
        setOutput("// error: no main function exported");
      }
    } catch (err) {
      setOutput(`// runtime error:\n// ${err}`);
    }

    setIsRunning(false);
  }, [code]);

  return (
    <div className="h-screen flex flex-col">
      {/* header */}
      <header className="h-12 border-b border-[#333333] flex items-center justify-between px-3 sm:px-4 shrink-0">
        <div className="flex items-center gap-2">
          {/* mobile menu button */}
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className="lg:hidden p-1 text-[#888888] hover:text-[#e0e0e0]"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
            </svg>
          </button>
          <Link href="/" className="flex items-center gap-1 text-lg font-bold">
            <span>.</span>
            <span>✱</span>
            <span className="hidden sm:inline">lang</span>
          </Link>
        </div>
        <button
          onClick={handleRun}
          disabled={isLoading || isRunning}
          className="px-3 sm:px-4 py-1 bg-[#333333] rounded hover:bg-[#444444] transition-colors text-sm disabled:opacity-50"
        >
          {isLoading ? "loading..." : isRunning ? "running..." : "run"}
        </button>
      </header>

      {/* mobile tab switcher */}
      <div className="lg:hidden flex border-b border-[#333333] shrink-0">
        <button
          onClick={() => setActiveTab("docs")}
          className={`flex-1 py-2 text-sm transition-colors ${
            activeTab === "docs" ? "text-[#e0e0e0] bg-[#252525]" : "text-[#888888]"
          }`}
        >
          docs
        </button>
        <button
          onClick={() => setActiveTab("code")}
          className={`flex-1 py-2 text-sm transition-colors ${
            activeTab === "code" ? "text-[#e0e0e0] bg-[#252525]" : "text-[#888888]"
          }`}
        >
          code
        </button>
      </div>

      <div className="flex-1 flex overflow-hidden relative">
        {/* mobile sidebar overlay */}
        {sidebarOpen && (
          <div
            className="lg:hidden fixed inset-0 bg-black/50 z-40"
            onClick={() => setSidebarOpen(false)}
          />
        )}

        {/* sidebar */}
        <aside
          className={`
            fixed lg:static inset-y-0 left-0 z-50
            w-48 bg-[#1a1a1a] border-r border-[#333333] p-4 overflow-y-auto shrink-0
            transform transition-transform lg:transform-none
            ${sidebarOpen ? "translate-x-0" : "-translate-x-full lg:translate-x-0"}
          `}
        >
          <nav className="flex flex-col gap-6 mt-12 lg:mt-0">
            {Object.entries(docs).map(([cat, items]) => (
              <div key={cat}>
                <h3 className="text-[#888888] text-xs uppercase tracking-wider mb-2">
                  {cat}
                </h3>
                <ul className="flex flex-col gap-1">
                  {items.map((item) => (
                    <li key={item.slug}>
                      <Link
                        href={`/playground?doc=${cat}/${item.slug}`}
                        onClick={() => setSidebarOpen(false)}
                        className={`block py-1 px-2 rounded text-sm transition-colors ${
                          category === cat && slug === item.slug
                            ? "bg-[#333333] text-[#e0e0e0]"
                            : "text-[#888888] hover:text-[#e0e0e0]"
                        }`}
                      >
                        {item.title}
                      </Link>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </nav>
        </aside>

        {/* docs panel - hidden on mobile when code tab active */}
        <div className={`flex-1 overflow-y-auto p-4 sm:p-6 border-r border-[#333333] ${activeTab === "code" ? "hidden lg:block" : ""}`}>
          <article className="prose prose-invert max-w-none">
            <div dangerouslySetInnerHTML={{ __html: parseMarkdown(docContent) }} />
          </article>
        </div>

        {/* editor + output panel - hidden on mobile when docs tab active */}
        <div className={`w-full lg:w-[45%] flex flex-col shrink-0 ${activeTab === "docs" ? "hidden lg:flex" : ""}`}>
          {/* editor */}
          <div className="flex-1 flex flex-col border-b border-[#333333] min-h-0">
            <div className="h-8 border-b border-[#333333] flex items-center px-3 text-xs text-[#888888] shrink-0">
              editor
            </div>
            <textarea
              value={code}
              onChange={(e) => setCode(e.target.value)}
              className="flex-1 bg-transparent p-3 sm:p-4 resize-none outline-none font-mono text-xs sm:text-sm min-h-50 lg:min-h-0"
              spellCheck={false}
            />
          </div>

          {/* output */}
          <div className="h-36 sm:h-48 flex flex-col shrink-0">
            <div className="h-8 border-b border-[#333333] flex items-center px-3 text-xs text-[#888888] shrink-0">
              output
            </div>
            <pre className="flex-1 p-3 sm:p-4 overflow-auto text-xs sm:text-sm text-[#888888]">
              {output || "// loading wasm modules..."}
            </pre>
          </div>
        </div>
      </div>
    </div>
  );
}

function parseMarkdown(md: string): string {
  return md
    .replace(/^### (.*$)/gm, '<h3 class="text-lg font-bold mt-6 mb-2">$1</h3>')
    .replace(/^## (.*$)/gm, '<h2 class="text-xl font-bold mt-8 mb-3">$1</h2>')
    .replace(/^# (.*$)/gm, '<h1 class="text-2xl font-bold mb-4">$1</h1>')
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.*?)\*/g, '<em>$1</em>')
    .replace(/`([^`]+)`/g, '<code class="bg-[#333333] px-1 rounded text-sm">$1</code>')
    .replace(/```(\w*)\n([\s\S]*?)```/g, '<pre class="bg-[#0a0a0a] p-4 rounded my-4 overflow-x-auto"><code>$2</code></pre>')
    .replace(/^\- (.*$)/gm, '<li class="ml-4">• $1</li>')
    .replace(/^\d+\. (.*$)/gm, '<li class="ml-4">$1</li>')
    .replace(/\n\n/g, '</p><p class="my-3">')
    .replace(/^(?!<[h|p|l|u|o|c|s|e|d])/gm, '')
    .replace(/^\s*$/gm, '');
}
