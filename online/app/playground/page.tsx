"use client";

import { Suspense, useEffect, useState, useRef, useCallback } from "react";
import { useSearchParams } from "next/navigation";
import Link from "next/link";
import ReactMarkdown from "react-markdown";
import mermaid from "mermaid";

mermaid.initialize({ startOnLoad: false, theme: "dark" });

function Mermaid({ chart }: { chart: string }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (ref.current) {
      const id = `mermaid-${Math.random().toString(36).slice(2)}`;
      mermaid.render(id, chart).then(({ svg }) => {
        if (ref.current) ref.current.innerHTML = svg;
      });
    }
  }, [chart]);

  return <div ref={ref} className="my-4" />;
}

const docs = {
  quickstart: [
    { slug: "intro", title: "introduction" },
    { slug: "syntax", title: "syntax" },
    { slug: "types", title: "types" },
    { slug: "functions", title: "functions" },
    { slug: "structs", title: "structs" },
  ],
  developer: [
    { slug: "intro", title: "introduction" },
    { slug: "architecture", title: "architecture" },
    { slug: "parser", title: "parser" },
    { slug: "typechecker", title: "type checker" },
    { slug: "codegen", title: "code generation" },
    { slug: "gc", title: "garbage collection" },
  ],
};

const algorithms: Record<string, string> = {
  "bubble sort": `fn main(): integer {
    let arr: {integer} = {5, 3, 8, 1, 2, 7, 4, 6};
    let n: integer = 8;
    for let i: integer = 0; i < n; i = i + 1; {
        for let j: integer = 0; j < n - i - 1; j = j + 1; {
            if arr[j] > arr[j + 1] {
                let temp: integer = arr[j];
                arr[j] = arr[j + 1];
                arr[j + 1] = temp;
            }
        }
    }
    for let i: integer = 0; i < n; i = i + 1; {
        print $arr[i];
    }
    return 0;
}`,
  "binary search": `fn binary_search(arr: {integer}, n: integer, target: integer): integer {
    let low: integer = 0;
    let high: integer = n - 1;
    while low <= high {
        let mid: integer = low + (high - low) / 2;
        if arr[mid] == target {
            return mid;
        }
        if arr[mid] < target {
            low = mid + 1;
        } else {
            high = mid - 1;
        }
    }
    return 0 - 1;
}

fn main(): integer {
    let arr: {integer} = {1, 3, 5, 7, 9, 11, 13, 15};
    let result: integer = binary_search(arr, 8, 7);
    print $result;
    return 0;
}`,
  "merge sort": `fn merge(arr: {integer}, left: integer, mid: integer, right: integer): integer {
    let temp: {integer} = {};
    let i: integer = left;
    let j: integer = mid + 1;
    while i <= mid and j <= right {
        if arr[i] <= arr[j] {
            temp = temp + {arr[i]};
            i = i + 1;
        } else {
            temp = temp + {arr[j]};
            j = j + 1;
        }
    }
    while i <= mid {
        temp = temp + {arr[i]};
        i = i + 1;
    }
    while j <= right {
        temp = temp + {arr[j]};
        j = j + 1;
    }
    for let k: integer = 0; k < right - left + 1; k = k + 1; {
        arr[left + k] = temp[k];
    }
    return 0;
}

fn merge_sort(arr: {integer}, left: integer, right: integer): integer {
    if left < right {
        let mid: integer = left + (right - left) / 2;
        merge_sort(arr, left, mid);
        merge_sort(arr, mid + 1, right);
        merge(arr, left, mid, right);
    }
    return 0;
}

fn main(): integer {
    let arr: {integer} = {8, 3, 5, 1, 9, 2, 7, 4};
    merge_sort(arr, 0, 7);
    for let i: integer = 0; i < 8; i = i + 1; {
        print $arr[i];
    }
    return 0;
}`,
  "fibonacci": `fn fibonacci(n: integer): integer {
    if n <= 1 {
        return n;
    }
    let a: integer = 0;
    let b: integer = 1;
    for let i: integer = 2; i <= n; i = i + 1; {
        let temp: integer = a + b;
        a = b;
        b = temp;
    }
    return b;
}

fn main(): integer {
    for let i: integer = 0; i < 10; i = i + 1; {
        print $fibonacci(i);
    }
    return 0;
}`,
  "primes to n": `fn main(): integer {
    let n: integer = 50;
    for let i: integer = 2; i <= n; i = i + 1; {
        let is_prime: boolean = true;
        for let j: integer = 2; j * j <= i; j = j + 1; {
            if i - (i / j) * j == 0 {
                is_prime = false;
            }
        }
        if is_prime {
            print $i;
        }
    }
    return 0;
}`,
};

const demos: Record<string, string> = {
  "quickstart/intro": `fn main(): integer {
    print "hello";
    print $(10 + 5);
    return 0;
}`,
  "quickstart/syntax": `fn main(): integer {
    let x: integer = 42;
    if x > 20 {
        print "big";
    } else {
        print "small";
    }
    let i: integer = 0;
    while i < 3 {
        print $i;
        i = i + 1;
    }
    return 0;
}`,
  "quickstart/types": `error MyError;

fn main(): integer {
    let nums: {integer} = {1, 2, 3};
    print $(nums[0]);

    fn maybe(): integer? {
        return 42;
    }
    print $(maybe()??);

    return 0;
}`,
  "quickstart/functions": `fn main(): integer {
    let x: integer = 10;

    fn add_x(y: integer): integer {
        return x + y;
    }

    fn factorial(n: integer): integer {
        if n <= 1 {
            return 1;
        }
        return n * factorial(n - 1);
    }

    print $(add_x(5));
    print $(factorial(5));
    return 0;
}`,
  "quickstart/structs": `struct Person {
    name: string,
    age: integer
}

fn main(): integer {
    let p: Person = new Person {
        name: "alice",
        age: 30
    };
    print p.name;
    print $p.age;
    return 0;
}`,
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
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [commandQuery, setCommandQuery] = useState("");
  const commandInputRef = useRef<HTMLInputElement>(null);

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

  // Load docs and demo code
  useEffect(() => {
    fetch(`/docs/${docPath}.md`)
      .then((r) => (r.ok ? r.text() : "# not found\n\nthis doc doesn't exist yet."))
      .then((md) => setDocContent(md))
      .catch(() => setDocContent("# error\n\nfailed to load doc."));

    if (demos[docPath]) {
      setCode(demos[docPath]);
    }
  }, [docPath]);

  // Command palette keyboard shortcut
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setCommandPaletteOpen((prev) => !prev);
        setCommandQuery("");
      }
      if (e.key === "Escape") {
        setCommandPaletteOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  useEffect(() => {
    if (commandPaletteOpen && commandInputRef.current) {
      commandInputRef.current.focus();
    }
  }, [commandPaletteOpen]);

  const filteredAlgorithms = Object.keys(algorithms).filter((name) =>
    name.toLowerCase().includes(commandQuery.toLowerCase())
  );

  const selectAlgorithm = (name: string) => {
    setCode(algorithms[name]);
    setCommandPaletteOpen(false);
    setCommandQuery("");
  };

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
          <Link href="/" className="flex items-center gap-1 text-lg font-bold font-mono">
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
          <article className="max-w-none">
            <ReactMarkdown
              components={{
                h1: ({ children }) => <h1 className="text-2xl font-bold mb-4">{children}</h1>,
                h2: ({ children }) => <h2 className="text-xl font-bold mt-8 mb-3">{children}</h2>,
                h3: ({ children }) => <h3 className="text-lg font-bold mt-6 mb-2">{children}</h3>,
                p: ({ children }) => <p className="my-3">{children}</p>,
                code: ({ className, children }) => {
                  if (className === "language-mermaid") {
                    return <Mermaid chart={String(children)} />;
                  }
                  return <code>{children}</code>;
                },
                pre: ({ children }) => {
                  // eslint-disable-next-line @typescript-eslint/no-explicit-any
                  const child = children as any;
                  if (child?.props?.className === "language-mermaid") {
                    return <>{children}</>;
                  }
                  return <pre className="p-4 my-4 overflow-x-auto">{children}</pre>;
                },
                ul: ({ children }) => <ul className="my-3 ml-4">{children}</ul>,
                ol: ({ children }) => <ol className="my-3 ml-4">{children}</ol>,
                li: ({ children }) => <li className="my-1">• {children}</li>,
              }}
            >
              {docContent}
            </ReactMarkdown>
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

      {/* command palette */}
      {commandPaletteOpen && (
        <div className="fixed inset-0 z-[100] flex items-start justify-center pt-[20vh]" onClick={() => setCommandPaletteOpen(false)}>
          <div className="fixed inset-0 bg-black/50" />
          <div
            className="relative w-full max-w-md bg-[#1a1a1a] border border-[#333333] rounded-lg shadow-2xl overflow-hidden"
            onClick={(e) => e.stopPropagation()}
          >
            <input
              ref={commandInputRef}
              value={commandQuery}
              onChange={(e) => setCommandQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && filteredAlgorithms.length > 0) {
                  selectAlgorithm(filteredAlgorithms[0]);
                }
              }}
              placeholder="Type an algorithm name..."
              className="w-full px-4 py-3 bg-transparent border-b border-[#333333] outline-none text-sm font-mono"
            />
            <ul className="max-h-60 overflow-y-auto">
              {filteredAlgorithms.map((name) => (
                <li key={name}>
                  <button
                    onClick={() => selectAlgorithm(name)}
                    className="w-full text-left px-4 py-2 text-sm hover:bg-[#333333] transition-colors"
                  >
                    {name}
                  </button>
                </li>
              ))}
              {filteredAlgorithms.length === 0 && (
                <li className="px-4 py-2 text-sm text-[#888888]">no matching algorithms</li>
              )}
            </ul>
          </div>
        </div>
      )}
    </div>
  );
}

