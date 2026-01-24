import Link from "next/link";
import Image from "next/image";

export default function Home() {
  return (
    <div className="min-h-screen">
      {/* hero */}
      <section className="min-h-screen flex flex-col items-center justify-center px-6">
        <Image
          src="/logo.png"
          alt=".*lang"
          width={320}
          height={80}
          priority
          className="mb-8"
        />

        <p className="text-[#888888] text-xl text-center max-w-2xl leading-relaxed mb-12">
          a statically-typed programming language for{" "}
          <span className="text-[#e0e0e0]">cross-language library development</span>
        </p>

        <div className="flex gap-6 mb-16">
          <Link
            href="/playground?doc=quickstart/intro"
            className="px-8 py-4 bg-[#e0e0e0] rounded font-bold hover:bg-white transition-all"
            style={{ color: "#1a1a1a" }}
          >
            get started
          </Link>
          <Link
            href="/playground?doc=developer/architecture"
            className="px-8 py-4 border border-[#333333] rounded hover:border-[#e0e0e0] hover:bg-[#222222] transition-all"
          >
            documentation
          </Link>
        </div>

        <div className="text-[#555555] text-sm animate-bounce">↓ scroll</div>
      </section>

      {/* the problem */}
      <section className="py-24 px-6 border-t border-[#333333]">
        <div className="max-w-5xl mx-auto">
          <h2 className="text-3xl font-bold mb-4 text-center">the problem</h2>
          <p className="text-[#888888] text-center max-w-2xl mx-auto mb-12">
            companies maintain separate sdks for every language. same functionality,
            different implementations. redundant effort, inconsistent behavior.
          </p>

          <div className="grid md:grid-cols-2 gap-6 mb-8">
            <div className="border border-[#333333] rounded-lg overflow-hidden">
              <div className="bg-[#252525] px-4 py-2 text-sm text-[#888888] border-b border-[#333333]">
                python sdk
              </div>
              <Image
                src="/adk.py.png"
                alt="Python SDK example"
                width={600}
                height={400}
                className="w-full"
              />
            </div>
            <div className="border border-[#333333] rounded-lg overflow-hidden">
              <div className="bg-[#252525] px-4 py-2 text-sm text-[#888888] border-b border-[#333333]">
                typescript sdk
              </div>
              <Image
                src="/adk.ts.png"
                alt="TypeScript SDK example"
                width={600}
                height={400}
                className="w-full"
              />
            </div>
          </div>

          <p className="text-[#666666] text-sm text-center">
            google&apos;s agent development kit — same api, maintained twice
          </p>
        </div>
      </section>

      {/* the solution */}
      <section className="py-24 px-6 border-t border-[#333333] bg-[#151515]">
        <div className="max-w-5xl mx-auto">
          <h2 className="text-3xl font-bold mb-4 text-center">the solution</h2>
          <p className="text-[#888888] text-center max-w-2xl mx-auto mb-16">
            write your library once in star. compile to webassembly. use from any language
            with a wasm runtime — python, javascript, rust, go, and more.
          </p>

          <div className="flex flex-col md:flex-row items-center justify-center gap-4 md:gap-8 mb-16">
            <div className="px-6 py-4 border border-[#333333] rounded-lg text-center">
              <div className="text-2xl mb-2">✱</div>
              <div className="text-sm text-[#888888]">write in star</div>
            </div>
            <div className="text-[#555555] text-2xl">→</div>
            <div className="px-6 py-4 border border-[#333333] rounded-lg text-center">
              <div className="text-2xl mb-2">⬡</div>
              <div className="text-sm text-[#888888]">compile to wasm</div>
            </div>
            <div className="text-[#555555] text-2xl">→</div>
            <div className="px-6 py-4 border border-[#333333] rounded-lg text-center">
              <div className="text-2xl mb-2">⊕</div>
              <div className="text-sm text-[#888888]">use anywhere</div>
            </div>
          </div>

          <div className="grid md:grid-cols-3 gap-4 text-center">
            <div className="p-4">
              <div className="text-[#e0e0e0] font-bold mb-2">one codebase</div>
              <div className="text-[#666666] text-sm">no more maintaining parallel implementations</div>
            </div>
            <div className="p-4">
              <div className="text-[#e0e0e0] font-bold mb-2">consistent behavior</div>
              <div className="text-[#666666] text-sm">same binary, same results, every language</div>
            </div>
            <div className="p-4">
              <div className="text-[#e0e0e0] font-bold mb-2">native feel</div>
              <div className="text-[#666666] text-sm">generated stubs provide idiomatic apis</div>
            </div>
          </div>
        </div>
      </section>

      {/* code example */}
      <section className="py-24 px-6 border-t border-[#333333]">
        <div className="max-w-3xl mx-auto">
          <h2 className="text-3xl font-bold mb-4 text-center">simple syntax</h2>
          <p className="text-[#888888] text-center mb-12">
            familiar c-style syntax with modern conveniences
          </p>

          <pre className="bg-[#0f0f0f] border border-[#333333] rounded-lg p-6 text-sm overflow-x-auto">
            <code>{`struct User {
    name: string,
    email: string
}

fn greet(user: User): string {
    return "hello, " + user.name;
}

fn main(): integer {
    let user: User = new User {
        name: "alice",
        email: "alice@example.com"
    };

    print greet(user);
    return 0;
}`}</code>
          </pre>
        </div>
      </section>

      {/* cta */}
      <section className="py-24 px-6 border-t border-[#333333] bg-[#151515]">
        <div className="max-w-2xl mx-auto text-center">
          <h2 className="text-3xl font-bold mb-4">ready to try?</h2>
          <p className="text-[#888888] mb-8">
            learn the language or dive into the internals
          </p>

          <div className="flex justify-center gap-6">
            <Link
              href="/playground?doc=quickstart/intro"
              className="px-8 py-4 bg-[#e0e0e0] rounded font-bold hover:bg-white transition-all"
              style={{ color: "#1a1a1a" }}
            >
              quick start
            </Link>
            <Link
              href="/playground?doc=developer/architecture"
              className="px-8 py-4 border border-[#333333] rounded hover:border-[#e0e0e0] hover:bg-[#222222] transition-all"
            >
              developer docs
            </Link>
          </div>
        </div>
      </section>

      {/* footer */}
      <footer className="py-8 border-t border-[#333333] text-center text-[#666666] text-sm">
        <a
          href="https://github.com/kavishsathia/star"
          target="_blank"
          rel="noopener noreferrer"
          className="hover:text-[#e0e0e0] transition-colors"
        >
          github
        </a>
        <span className="mx-4">·</span>
        <span>mit license</span>
      </footer>
    </div>
  );
}
