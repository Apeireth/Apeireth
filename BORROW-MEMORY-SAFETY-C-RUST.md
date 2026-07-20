## Medium

**Source**: https://medium.com/@shyamsundarb/memory-safety-in-c-vs-rust-vs-zig-f78fa903f41e

---

[Sitemap](/sitemap/sitemap.xml)

[Open in app](https://play.google.com/store/apps/details?id=com.medium.reader&referrer=utm_source%3DmobileNavBar&source=---top_nav_layout_nav-----------------------------------------)

[Sign in](/m/signin?operation=login&redirect=https%3A%2F%2Fmedium.com%2F%40shyamsundarb%2Fmemory-safety-in-c-vs-rust-vs-zig-f78fa903f41e&source=post_page---top_nav_layout_nav-----------------------global_nav------------------)

[Medium Logo](/?source=---top_nav_layout_nav-----------------------------------------)

[Write](/m/signin?operation=register&redirect=https%3A%2F%2Fmedium.com%2Fnew-story&source=---top_nav_layout_nav-----------------------new_post_topnav------------------)

[Search](/search?source=---top_nav_layout_nav-----------------------------------------)

[Sign in](/m/signin?operation=login&redirect=https%3A%2F%2Fmedium.com%2F%40shyamsundarb%2Fmemory-safety-in-c-vs-rust-vs-zig-f78fa903f41e&source=post_page---top_nav_layout_nav-----------------------global_nav------------------)

Cpp

Rust

Zig

Memory Safety

Performance

# Memory Safety in C++ vs Rust vs Zig

[/@shyamsundarb?source=post_page---byline--f78fa903f41e---------------------------------------](/@shyamsundarb?source=post_page---byline--f78fa903f41e---------------------------------------)

[B Shyam Sundar](/@shyamsundarb?source=post_page---byline--f78fa903f41e---------------------------------------)

18 min read

·

Jul 6, 2024

[/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2F_%2Fvote%2Fp%2Ff78fa903f41e&operation=register&redirect=https%3A%2F%2Fmedium.com%2F%40shyamsundarb%2Fmemory-safety-in-c-vs-rust-vs-zig-f78fa903f41e&user=B+Shyam+Sundar&userId=d0592a299320&source=---header_actions--f78fa903f41e---------------------clap_footer------------------](/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2F_%2Fvote%2Fp%2Ff78fa903f41e&operation=register&redirect=https%3A%2F%2Fmedium.com%2F%40shyamsundarb%2Fmemory-safety-in-c-vs-rust-vs-zig-f78fa903f41e&user=B+Shyam+Sundar&userId=d0592a299320&source=---header_actions--f78fa903f41e---------------------clap_footer------------------)

--

[/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2F_%2Frepost%2Fp%2Ff78fa903f41e&operation=register&redirect=https%3A%2F%2Fmedium.com%2F%40shyamsundarb%2Fmemory-safety-in-c-vs-rust-vs-zig-f78fa903f41e&user=B+Shyam+Sundar&userId=d0592a299320&source=---header_actions--f78fa903f41e---------------------repost_header------------------](/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2F_%2Frepost%2Fp%2Ff78fa903f41e&operation=register&redirect=https%3A%2F%2Fmedium.com%2F%40shyamsundarb%2Fmemory-safety-in-c-vs-rust-vs-zig-f78fa903f41e&user=B+Shyam+Sundar&userId=d0592a299320&source=---header_actions--f78fa903f41e---------------------repost_header------------------)

[/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2F_%2Fbookmark%2Fp%2Ff78fa903f41e&operation=register&redirect=https%3A%2F%2Fmedium.com%2F%40shyamsundarb%2Fmemory-safety-in-c-vs-rust-vs-zig-f78fa903f41e&source=---header_actions--f78fa903f41e---------------------bookmark_footer------------------](/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2F_%2Fbookmark%2Fp%2Ff78fa903f41e&operation=register&redirect=https%3A%2F%2Fmedium.com%2F%40shyamsundarb%2Fmemory-safety-in-c-vs-rust-vs-zig-f78fa903f41e&source=---header_actions--f78fa903f41e---------------------bookmark_footer------------------)

[Listen](/m/signin?actionUrl=https%3A%2F%2Fmedium.com%2Fplans%3Fdimension%3Dpost_audio_button%26postId%3Df78fa903f41e&operation=register&redirect=https%3A%2F%2Fmedium.com%2F%40shyamsundarb%2Fmemory-safety-in-c-vs-rust-vs-zig-f78fa903f41e&source=---header_actions--f78fa903f41e---------------------post_audio_button------------------)

A look at C++, Zig and Rust in terms of memory safety

Press enter or click to view image in full size

C++ | Zig | Rust

## TL; DR

I intend to articulate about memory-safety with C++, Zig and Rust at a very basic level. The structure of this article is based on the work of [Sean Baxter](https://x.com/seanbax) and his [Circle C++](https://www.circle-lang.org/site/intro/) compiler.

**My conclusions:**

Generally, C++ lets the programmer be free and do whatever they want. With Circle C++ offers a compelling answer for enhancing C++ to be stricter with memory safety and offers excellent add-on feature to C++ which can be easily and most importantly progressively adapted to existing C++ codebases.

Rust offers exceptional defaults and strict memory safety. It truly is an excellent programming language, but it also has a tough learning curve as concepts like borrow-checker might be an alien concept to C++ veterans.

Zig offers balance, reasonably memory-safety (hands-off approach with allocators) and overall, a much simpler language to pick in comparison to C++ or Rust. It also offers excellent integration with C/C++ codebases which lets it be added quite easily to existing codebases.

## My views on the languages in terms of memory safety

### C++

Though modern C++ (C++11, C++14, …) has introduced a lot of features for safety, still it is quite easy to write code which causes memory issues not to mention UB (undefined behaviour) more on this later in the article. It is no surprise that C++ right has gotten a bad rapport in the industry in terms of safety.

One thing that we can all agree is that the sheer amount of legacy code available in C++ and the number of experienced C++ developers is huge. In any organization that has a huge C++ codebase along with an experienced C++ developer base, pivoting to an entirely new language is quite tough if not impossible, time-consuming and in-turn a costly affair. The cost aspect of reskilling and rewrite alone might be a deal breaker for a lot of organizations. But things are all not that bad. Sean Baxter with his Circle compiler is coming out with a practical way to address these concerns.

### Zig

Zig, the relatively new kid in the block offers a unique way to manage memory with a hands-off approach to allocation. While memory safety is not as strict as with Rust, Zig does provide an intuitive pattern of being quite explicit as to when memory is allocated to the heap. Zig hasn’t hit 1.0 yet, still it is being used in production by ambitious projects like [Bun](https://bun.sh/) and [Tigerbeetle](https://tigerbeetle.com/).

### Rust

I am a big fan of Rust. Rust is an excellent choice as it is well known to have rigorous safety standards in their code with use of borrow-checker and an excellent compiler. Rust has now gotten mainstream adoption as well, with support for it getting added to the [Linux Kernal](https://www.theregister.com/2022/10/05/rust_kernel_pull_request_pulled/) version 6.1 onwards and companies like [Amazon](https://aws.amazon.com/blogs/opensource/sustainability-with-rust/), [Microsoft](https://x.com/markrussinovich/status/1803177945269252224) and [Google](https://opensource.googleblog.com/2021/02/google-joins-rust-foundation.html), all backing Rust. The future for Rust is quite bright.

## Setup

### C++

I will be using [Circle C++, Build 206](https://www.circle-lang.org/linux/build_206.tgz) for Safe C++ and g++ (GCC) 14.1.1 20240522 for the regular C++.

I used the [poac](https://poac.dev/) tool as a build system and package manager. It is inspired by the cargo tool of rust. Like rust’s cargo it expects the project to adhere to a certain structure. In return you needn’t be overwhelmed with the open-ended nature of setting up a complex build system like CMake. This project is under development and certainly lacks many features of the advanced build systems available for C++. For our article this tool should be sufficient.

### Zig

I will be using the [zig compiler version 0.12.1](https://ziglang.org/download/#:~:text=45.0MiB-,0.12.1,-2024%2D06%2D08) for compiling Zig code. I use the zig-init command to initiate the repo and will use zig as build-system and package manager.

### Rust

I will be using [rustc 1.79.0 (129f3b996 2024–06–10)](https://blog.rust-lang.org/2024/06/13/Rust-1.79.0.html) for compiling Rust. I used the provided cargo tool as a build system and package manager.

## How am I going to compare memory safety

Based on Sean’s work, memory safety can be broken down into five categories:

1. Lifetime safety
2. Type safety — null variety
3. Type safety — union variety
4. Thread safety
5. Runtime safety

For the sake of this article, I am only going to look at lifetime safety, type safety (null and union) and runtime safety.

## Lifetime Safety

To understand lifetime safety let us look at how the “borrow checking” works each of these languages.

### C++

Consider the following code in C++

```
#include <cstdio>int main() {  int* a;  {    int b = 10;    a = &b; }  int c = *a;  printf("%d\n", c);  *a = 11;  printf("%d\n", *a);}
```

When compiled with g++ and executed we get the following output:

g++ use after free

The value 10 is printed. This behaviour is memory unsafe, and this problem is known as use-after-free (UAF). The value of “10” is held by the variable “b”. The variable b went out of scope on the 8th line’s “}”. “a” is pointer which points to the address of “b” which went out of scope. But still in line 9, I am able to dereference “a”. Also in line 11, I am also able to mutate the contents. [UAF is dangerous](https://encyclopedia.kaspersky.com/glossary/use-after-free/) and can cause disastrous effects.

To mitigate this problem modern C++ offers smart pointers. Consider the following code.

```
#include <cstdio>#include <memory>int main() {  std::unique_ptr<int> a;  {    int b = 10;    a = std::make_unique<int>(b);  }  int c = *a;  printf("%d\n", c);  *a = 11;  printf("%d\n", *a);}
```

The output which you will get is

g++ smart pointer

Why is this not UAF:

- The key is that `std::unique_ptr` manages the lifetime of the dynamically allocated object.
- Even though `b` goes out of scope, the memory is not immediately deallocated because `a` still owns it.
- Only when `a` itself goes out of scope (in the `main()` function) will the memory be released.

Now, let us see how Circle C++ deals with this. Consider the following code:

```
#feature on safety#include <cstdio>int main() safe {  int ^a; // An uninitialized borrow  {    int b = 10; // b is live    a = ^b; // though it seems a is mutably borrowing from b, within the current scope it isn't used and hence a is uninitialzed.  }  int c = *a;  unsafe printf("%d\n", c);  *a = 11;  unsafe printf("%d\n", *a);}
```

2 key things to note here are the following:

1. `#feature on safety`: This is a feature of the Circle C++ compiler, wherein we add specific features of the Circle C++ compiler.
2. The `safe `qualifier for the main function. This lets us specify safety for the particular wise. This I believe is an excellent way to introduce strict safety to existing large C++ codebases. This would allow for incrementally adding strict safety standard to existing projects without major complete rewrites which would

When we compiled with Circle C++ compiler, we get a compiler error as follows:

Circle C++ borrow checking

The `^` is a checked reference. Meaning at compile time, the borrows are checked whether if it is in scope or not. Thus, we get the safest behaviour of the compile step failing.

### Zig

In Zig, consider the following code:

```
const std = @import("std");pub fn main() !void {    var a: *i32 = undefined;    {        var b: i32 = 10;        a = &b;        std.debug.print("address of b is: {s}\n", .{&b});    }    std.debug.print("a is pointing to: {s}\n", .{a});    const c: i32 = a.*;    std.debug.print("{d}\n", .{c});    a.* = 11;    std.debug.print("{d}\n", .{a.*});}
```

This builds and outputs as:

zig — UAF

This result indicates that there clearly is use-after-free vulnerability in the code as the memory address “b” is still held by “a” even after “b” goes out of scope. To mitigate, Zig generally advises the following

1. Do not use pointers for stack allocated variables
2. Use allocators to allocate memory on the heap

Let us try to alter the code as:

```
const std = @import("std");pub fn main() !void {    var gpa = std.heap.GeneralPurposeAllocator(.{ .safety = true }){};    defer {        _ = gpa.deinit();    }    const allocator = gpa.allocator();    var a: *i32 = undefined;    {        var b = try allocator.create(i32);        b.* = 10;        a = b;        std.debug.print("address of b is: {s}\n", .{&b});    }    std.debug.print("a is pointing to: {s}\n", .{a});    const c: i32 = a.*;    std.debug.print("{d}\n", .{c});    a.* = 11;    std.debug.print("{d}\n", .{a.*});}
```

When we build and run this code we get:

Press enter or click to view image in full size

zig — Allocator

The program ran and the UAF did occur, however when the program ended and the allocator was about to go out of scope, the safety checks kicked in and informed us that the program is ill formed and there is a memory leak. To fully prevent UAF, we can slightly alter our code as follows:

```
const std = @import("std");pub fn main() !void {    var gpa = std.heap.GeneralPurposeAllocator(.{ .safety = true }){};    defer {        _ = gpa.deinit();    }    const allocator = gpa.allocator();    var a: *i32 = undefined;    {        var b = try allocator.create(i32);        defer allocator.destroy(b);        b.* = 10;        a = b;        std.debug.print("address of b is: {s}\n", .{&b});    }    std.debug.print("a is pointing to: {s}\n", .{a});    const c: i32 = a.*;    std.debug.print("{d}\n", .{c});    a.* = 11;    std.debug.print("{d}\n", .{a.*});}
```

The output is:

Press enter or click to view image in full size

zig — alloc.destroy()

Notice that segmentation fault occurred, and the program panicked when we tried dereferencing a. This happened because of `defer allocator.destroy(b);` This is because in with the `defer` the function mentioned there is executed once the current scope is exited. With this we can ensure that UAF is avoided.

### Rust

Let us consider the following:

```
fn main() {    let a:i32;    let a_ptr = &a;}
```

First, the aforementioned code will not compile because `a` is uninitialized. To understand this better, in Rust a pointer can never point to null or can never be uninitialized. When we try to compile the aforementioned code it results in

Press enter or click to view image in full size

Rust- uninitialzed

Now let us consider the following:

```
fn main() {    let mut a = 10;    let mut a_ptr: *mut i32 = &mut a;    {        let mut b = 11;        a_ptr = &mut b;    }    let c = *a_ptr;}
```

The aforementioned code will not compile. This is because of the line `let c = *a_ptr` . In Rust, dereferencing a raw pointer is considered unsafe behaviour and the programmer is expected to isolate that code into an `unsafe` block if needs be. Rust makes these base scenarios inherently safer when compared to C++ or Zig.

Press enter or click to view image in full size

Rust — Raw pointer dereference

## Type safety — null and default value

### C++

Types that have default value when constructed. For example, `std::unique_ptr`has a default value of 0 when constructed. If a * or -> operator is used the output you will get is something equivalent to an undefined behaviour essentially, it is a reference to a `nullptr`.

Consider the following code:

```
#include <iostream>#include <memory>#include <string>using namespace std;int main() {  unique_ptr<string> a;  cout << *a;}
```

When compiling this code with g++ and executing it we get a crash with SIGSEGV (Address Boundary Error).

Press enter or click to view image in full size

g++ unique_ptr

This is because a is uninitialized memory, and just dereferencing it caused it to read from memory which hasn’t been properly allocated. This is undefined behaviour and has caused the crash. While a program crashing with a segmentation fault (SIGSEGV) might seem like it’s “safe” in the sense that it prevents further execution, it doesn’t necessarily mean the program is secure. A crash (like SIGSEGV) occurs when the program accesses memory it shouldn’t. It’s a sign of undefined behaviour, but it doesn’t guarantee safety. Security vulnerabilities can still exist even if the program crashes. For example, an attacker might intentionally trigger a crash to disrupt the system or gather information.

With Circle C++, this behaviour is curtailed at compile time itself which is the correct. At the time of writing this article the `std2.h` has not been released for public use yet and hence I am pointing you to the Circle C++’s website.

Circle C++

The compile time check occurs because of the following quote from the Circle C++ website itself.

> The `std2::unique_ptr` has no default state. It's safe against [null pointer type safety bugs](https://www.circle-lang.org/site/intro/#2-type-safety-null-variety). A `unique_ptr` that's declared without a constructor is not default initialized. It's uninitialized. It's illegal to use until it's been assigned to.

### Zig

Consider the following code:

```
const std = @import("std");pub fn main() !void {    const a: *[]u8 = undefined;    std.debug.print("{s}\n", .{a.*});}
```

When compiling this we get:

Press enter or click to view image in full size

zig build — undefined

Here zig correctly fails to compile because there is no default state. In Zig, the there is an optional type which you have explicitly mention. By default, undefined type cannot be dereferenced, and it is caught at compile time itself.

### Rust

Consider the following code:

```
fn main() {    let a: &i32;    println!("{}", *a);}
```

When we try to compile the aforementioned code, we get

Press enter or click to view image in full size

Rsut — uninitialized

In Rust every variable needs to be initialized and there is no default state. If the variable can have optional the `Option<T>` type is supposed to used and explicit unwrap must be done. If this behaviour needs to be bypassed, then we need to enter into unsafe context.

## Type safety — union and choice types

We will have a look at how each of the programming languages handles choice type and pattern matching.

### C++

In C++ there are two main types for us to consider, they are `std::optional`, `std::variant`and `std::expected`. Let have a look the following code and understand how safe they are with g++ and circle C++.

Consider the following code.

```
#include <iostream>#include <optional>#include <random>#include <string>using namespace std;std::optional<string> maybe_string() {  std::random_device rd;  std::mt19937 gen(rd());  std::uniform_int_distribution<> distr(1, 100);  auto r = distr(gen);  cout << "rand val: " << r << '\n';  if (r % 2) {    return "Hello";  } else {    return nullopt;  }}int main() {  optional<string> a = maybe_string();  cout << *a << '\n';}
```

When we compile this program with g++ and execute it we get:

Press enter or click to view image in full size

When the random value is odd then it simply prints “hello” which is the correct behaviour. However, when an even number is generated then it leads to a nullopt being dereferenced which unsafe and leads to an SIGSEGV.

Let us consider a simpler code:

```
#include <iostream>#include <optional>#include <string>using namespace std;int main() {  optional<string> a;  cout << *a << '\n';}
```

When we execute this, we get:

Press enter or click to view image in full size

g++ optional

The optional `a` has not been initialized. When we dereference this with `*a` is undefined behaviour and is considered as unsafe.

For Circle C++, consider the following code:

```
#feature on safety#include <cstdio>#include <optional>#include <string>using namespace std;int main() safe {  optional<string> a;  unsafe printf("%s\n", *a);}
```

When we compile it, we get:

Circle C++ optional

We correctly get a compile error with proper error message of an uninitialized object. This is a safe behaviour.

Let us see how the `std::variant` type works with a sample code.

```
#include <variant>#include <string>#include <iostream>using namespace std;int main() {  variant<string, int> a;  string b = get<string>(a);  cout << b << '\n';}
```

When compiled with g++ and executed we get:

g++ variant

Like `optional` when we try to get uninitialized memory, we get undefined behaviour which is unsafe.

With circle C++ we slightly can alter the code as:

```
#feature on safety#include <cstdio>#include <variant>#include <string>using namespace std;int main() safe {  variant<string, int> a;  string b = get<string>(a);  unsafe printf("%s\n", b);}
```

In this the Circle C++ compiler would not let us use the `get<T>()` function in the safe context and will stop compiling. This certainly is a safe behaviour.

Press enter or click to view image in full size

circle c++ variant

> Note that the Circle C++ compiler offers safe standard library called `std2` which offers a safe way of using `optional` and `variant`

### Zig

In Zig, there is `union` and `optional` type. Beyond this, the return type of a function can return an `error!success` which is a result type and has to be handled safely.

Let us consider the following code:

```
const std = @import("std");pub fn main() !void {    const A = union { int: i32, is: bool };    const variant = A{};    std.debug.print("d has: {any}", .{variant});}
```

When we compile this, we get:

Press enter or click to view image in full size

zig union

In Zig the union does not have a default state and hence will not let you use it until you properly initialize it.

Let us look at how `optionals` work in Zig. Consider the following code:

```
const std = @import("std");pub fn main() !void {    const a: ?*i32 = null;    std.debug.print("a has: {any}", .{a.?.*});}
```

`Optionals` use the syntax `?T` and are used to store the data `[null](https://ziglang.org/documentation/master/#null)`, or a value of type `T`. When we try to build the aforementioned code, we get:

Press enter or click to view image in full size

zig — optionals

The build fails at compile time itself. This is a safe behaviour.

### Rust

In Rust we have the `Result`, `Option` and `enum` type.

We will first look at `Option` type. Consider the following code:

```
fn main() {    let a:Option<String>;    println!("{:?}", a);}
```

The aforementioned code won’t compile, we get

Press enter or click to view image in full size

This is because Rust by default does not allow null or uninitialized values.

Let us have a look at `enums`. Consider the following code:

```
fn main() {    enum A {        Int(i32),        Is(bool),    }    let variant = A::Int(10);}
```

Rust does not have a default state for the enum `A`. Hence, without specifying an exclusive value for the `enum`. We cannot get an instance of the `enum`. Further, Rust’s `enum` matching is exhaustive. Let us extend the aforementioned code as:

```
fn main() {    enum A {        Int(i32),        Is(bool),    }    let variant = A::Int(10);    match variant {        A::Int(_) => todo!()    }}
```

Press enter or click to view image in full size

rust enum match

The compiler forces you to handle all valid states of the `enum` to ensure that all states are exclusively handled.

## Runtime safety

Under runtime safety we can consider the following scenarios out-of-bounds and divide-by-zero.

### Out-Of-Bounds

### C++

Consider the following code.

```
#include <iostream>int main() {  int a[10] = {};  size_t index = 15;  // We use an out of bounds index to access here.   int b = a[index];  std::cout << b << '\n';}
```

If we compile this with the g++ compiler and run the output, we get some random number. This is undefined behaviour.

g++ default

The UB can be circumvented by utilizing sanitizers provided by the compilers. This ensures that during runtime UB is avoided and the program panics and exists. However, relying solely on sanitizers isn’t an advisable behaviour as they aren’t exactly consistent at catching out-of-bounds all the time.

Press enter or click to view image in full size

g++ with AddressSanitizer

The modern C++ does provide a “safe” way to work with arrays, let us slightly alter the code:

```
#include <iostream>#include <vector>int main() {  std::vector<int> a;  size_t index = 15;  // We use an out of bounds index here.   int b = a[index];  std::cout << b << '\n';}
```

With the use of vector, when we compile and run the program we get program termination:

Press enter or click to view image in full size

g++ with std::vector container

Additionally, when we use the `.at()` to access it does explicit bounds checking before accessing it.

```
#include <iostream>#include <vector>int main() {  std::vector<int> a;  size_t index = 15;  // We use an out of bounds index here.   int b = a.at(index);  std::cout << b << '\n';}
```

We get a proper behaviour of std::out_of_range exception.

Press enter or click to view image in full size

g++ with std::vector and at accessor

Let us see what circle C++ compiler has to offer. Consider the following code.

```
#feature on safety#include <cstdio>int main() safe {  int a[10] = {};  size_t index = 15;  // We use an out of bounds index here.   int b = a[index];  unsafe printf("%d\n", b);}
```

When we compile it with circle and run it, we get the following output:

Press enter or click to view image in full size

Circle C++ default

This gave a clean output for out-of-range, with the regular array itself without using any sanitizers or standard library.

### Zig

Consider the following code

```
const std = @import("std");pub fn main() !void {    const a: [10]i32 = undefined;    const index = 15;    const b = a[index];    std.debug.print("{d}", .{b});}
```

Like rust arrays the build itself would fail with checks.

Press enter or click to view image in full size

zig default build

Should we try to use a container type similar to C++ vectors then we can consider the following code.

```
const std = @import("std");pub fn main() !void {    var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);    defer arena.deinit();    const allocator = arena.allocator();    const a = std.ArrayList(i32).init(allocator);    const index = 15;    const b = a.items[index];    std.debug.print("{d}", .{b});}
```

The program correctly panics and exits with out of bounds error.

Press enter or click to view image in full size

zig — arraylist

Though Zig’s default memory guard rails are good, it does have some rough edges. Consider:

```
const std = @import("std");pub fn main() !void {    const a: [10]i32 = undefined;    for (a) |x| {        std.debug.print("value: {d}\n", .{x});    }}
```

The above-mentioned code results in:

zig — undefined

This gives an output without throwing an error. This is actually reading data from uninitialized memory which is considered memory unsafe and is an undefined behaviour. Refer Zig’s [official documentation](https://ziglang.org/documentation/master/#undefined:~:text=undefined%20can%20be,before%20being%20used.%22)on what `undefined `means and how to handle it.

### Rust

Consider the following code.

```
fn main() {    let a: [i32; 10] = Default::default();    let index: usize = 15;    let b = a[index];    println!("{}", b);}
```

The build itself would fail with index out of bounds error. This is thanks to the rust compilers exhaustive static checking capabilities.

Press enter or click to view image in full size

rustc default

Still, we can bypass the build checking by making use of vector. Consider the following code:

```
fn main() {    let a = vec![1,2,3];    let _ = a[3];}
```

This code will pass the build checks, however when we run this, the program panics with out of bounds. As to why this passes the build, I encourage you to check this [Stack overflow answer](https://stackoverflow.com/a/24971990/3528284) as it gives a concise explanation as to why.

Press enter or click to view image in full size

rustc with vec

Let us see how Rust handles uninitlized memory, consider the following:

```
fn main() {    let a:Vec<i32> = Vec::with_capacity(3);    let _ = a[3];}
```

When running this program, we correctly get index out of bounds panic.

Press enter or click to view image in full size

rustc with vec uninitalized

By default, Rust has much better guardrails to save you from memory related issues.

Also, Rust has much stricter checking in place in terms of not letting you access undefined memory region.

## Final thoughts

This article is no means a comprehensive look at the memory management models of the respective languages. What I want to present is a fair and basic view of memory safety works in each language.

C++ language philosophy permits you the broadest spectrum of behaviour and really has all the bells and whistles. The programmer has the **freedom** to choose their style and must implement things as they see fit which may offer the best memory safety model or the worst depending on programmer and project.

Zig offers **balance** between C++’s freedom and Rust’s rigid opiniated memory management model and hence has fairly strong memory safety and really a joy to code. Picking up the language is also relatively simple and has gentler learning curve and offers intuitive features while is gently nudges towards writing memory safe code.

Rust certainly offers strong memory safety guarantees and thus the compiler is quite strict and is quite in **control**. Beyond the basics the moment you get into lifetimes and async programming things become really tough and the language becomes a tough nut to crack.

## Further Reading

You can read about his work at [Circle-Lang](https://www.circle-lang.org/site/intro/) and presentation about memory safe C++ [here](https://www.youtube.com/watch?v=5Q1awoAwBgQ).

[Unsafe Zig is Safer Than Unsafe Rust - Andrew KelleyConsider the following Rust code: struct Foo { a: i32, b: i32, } fn main() { unsafe { let mut array: [u8; 1024] = [1…andrewkelley.me](https://andrewkelley.me/post/unsafe-zig-safer-than-unsafe-rust.html?source=post_page-----f78fa903f41e---------------------------------------)

[`zig cc`: a Powerful Drop-In Replacement for GCC/ClangIf you have heard of Zig before, you may know it as a promising new programming language which is ambitiously trying to…andrewkelley.me](https://andrewkelley.me/post/zig-cc-powerful-drop-in-replacement-gcc-clang.html?source=post_page-----f78fa903f41e---------------------------------------)

[Cpp](/tag/cpp?source=post_page---footer_tags--f78fa903f41e---------------------------------------)

[Rust](/tag/rust?source=post_page---footer_tags--f78fa903f41e---------------------------------------)

[Zig](/tag/zig?source=post_page---footer_tags--f78fa903f41e---------------------------------------)

[Memory Safety](/tag/memory-safety?source=post_page---footer_tags--f78fa903f41e---------------------------------------)

[Performance](/tag/performance?source=post_page---footer_tags--f78fa903f41e---------------------------------------)

[/@shyamsundarb?source=post_page---post_author_info--f78fa903f41e---------------------------------------](/@shyamsundarb?source=post_page---post_author_info--f78fa903f41e---------------------------------------)

[/@shyamsundarb?source=post_page---post_author_info--f78fa903f41e---------------------------------------](/@shyamsundarb?source=post_page---post_author_info--f78fa903f41e---------------------------------------)

[Written by B Shyam Sundar](/@shyamsundarb?source=post_page---post_author_info--f78fa903f41e---------------------------------------)

[272 followers](/@shyamsundarb/followers?source=post_page---post_author_info--f78fa903f41e---------------------------------------)

·

[13 following](/@shyamsundarb/following?source=post_page---post_author_info--f78fa903f41e---------------------------------------)

Software Architect at Novac Technology Solutions

[Help](https://help.medium.com/hc/en-us?source=post_page-----f78fa903f41e---------------------------------------)

[Status](https://status.medium.com/?source=post_page-----f78fa903f41e---------------------------------------)

[About](/about?autoplay=1&source=post_page-----f78fa903f41e---------------------------------------)

[Careers](/jobs-at-medium/work-at-medium-959d1a85284e?source=post_page-----f78fa903f41e---------------------------------------)

[Press](mailto:pressinquiries@medium.com)

[Blog](https://blog.medium.com/?source=post_page-----f78fa903f41e---------------------------------------)

[Store](https://medium.com/store)

[Privacy](https://policy.medium.com/medium-privacy-policy-f03bf92035c9?source=post_page-----f78fa903f41e---------------------------------------)

[Rules](https://policy.medium.com/medium-rules-30e5502c4eb4?source=post_page-----f78fa903f41e---------------------------------------)

[Terms](https://policy.medium.com/medium-terms-of-service-9db0094a1e0f?source=post_page-----f78fa903f41e---------------------------------------)

[Text to speech](https://speechify.com/medium?source=post_page-----f78fa903f41e---------------------------------------)