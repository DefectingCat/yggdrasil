-- 种子数据：高质量测试文章
-- 共 17 篇技术文章，覆盖 15+ 种编程语言

-- 先创建测试用户（如果不存在）
INSERT INTO users (username, email, password_hash, role)
VALUES ('testuser', 'test@example.com', '$argon2id$v=19$m=65536,t=3,p=4$testtesttesttesttesttesttest$testtesttesttesttesttesttesttesttesttest', 'admin')
ON CONFLICT (username) DO NOTHING;

-- 插入测试文章
INSERT INTO posts (author_id, title, slug, summary, content_md, content_html, status, published_at, created_at, updated_at)
VALUES
(
    1,
    'Rust 所有权与生命周期：深入理解内存安全',
    'rust-ownership-lifetime',
    '本文深入探讨 Rust 的所有权系统、借用规则和生命周期注解，帮助你理解 Rust 如何在不使用垃圾回收器的情况下保证内存安全。',
    $doc$
# Rust 所有权与生命周期：深入理解内存安全

Rust 是一门专注于安全、并发和性能的系统级编程语言。自 2010 年 Mozilla 研究院发起以来，Rust 凭借其独特的内存安全保证机制，逐渐在系统编程领域占据重要地位。与 C/C++ 等语言不同，Rust 不需要依赖垃圾回收器（Garbage Collector），而是在编译期通过**所有权系统**（Ownership System）来确保内存安全。这一设计使得 Rust 程序既拥有接近 C 语言的性能，又避免了手动内存管理带来的种种安全隐患。

本文将系统性地介绍 Rust 的所有权、借用和生命周期三大核心概念，帮助你建立对 Rust 内存模型的深入理解。

## 为什么需要所有权？

在传统的系统编程语言中，内存管理通常面临两种选择：手动管理（如 C/C++）或自动垃圾回收（如 Java、Go）。手动管理虽然性能优秀，但容易引发内存泄漏、悬垂指针、双重释放等严重问题；垃圾回收虽然安全，但会带来运行时开销和不可预测的暂停时间。

Rust 提出了第三种方案：**在编译期通过所有权规则静态验证内存访问的安全性**。这意味着所有内存安全检查都在编译阶段完成，运行时几乎零开销。

## 所有权的三条基本规则

Rust 的所有权系统建立在三条简单但强大的规则之上：

1. **每个值在任意时刻都有且仅有一个所有者（owner）**
2. **当所有者离开作用域时，其拥有的值会被自动释放**
3. **所有权可以通过移动（move）或复制（copy）在不同变量之间转移**

### 所有权转移示例

```rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1; // s1 的所有权转移给 s2
    
    // println!("{}", s1); // 编译错误！s1 已无效
    println!("{}", s2); // 正确，s2 拥有该字符串
}
```

在上述代码中，`String::from("hello")` 在堆上分配了一块内存。当执行 `let s2 = s1` 时，Rust 执行的是**移动语义**（move semantics）—— `s1` 将堆内存的所有权转移给 `s2`，之后 `s1` 不再有效。这种设计避免了双重释放问题。

### Copy trait 与 Clone trait

并非所有类型都会执行移动语义。对于栈上分配的简单类型（如整数、浮点数、布尔值等），Rust 会自动实现 `Copy` trait，这意味着赋值时执行的是**按位复制**而非所有权转移：

```rust
fn main() {
    let x = 5;
    let y = x; // i32 实现了 Copy，x 仍然有效
    
    println!("x = {}, y = {}", x, y); // 完全正确
}
```

对于需要深拷贝的复杂类型，可以使用 `clone()` 方法：

```rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1.clone(); // 显式深拷贝
    
    println!("s1 = {}, s2 = {}", s1, s2); // 两者都有效
}
```

## 借用（Borrowing）：安全地共享数据

虽然所有权转移解决了内存安全问题，但在实际编程中，我们经常需要临时访问某个值而不获取其所有权。Rust 提供了**借用**机制来解决这个问题。

借用分为两种类型：不可变借用和可变借用。

### 不可变借用（Immutable Borrowing）

不可变借用允许你读取数据但不修改它。在同一作用域内，可以创建多个不可变引用：

```rust
fn calculate_length(s: &String) -> usize {
    s.len()
}

fn main() {
    let s = String::from("hello");
    let len = calculate_length(&s); // 传递不可变引用
    
    println!("'{}' 的长度是 {}.", s, len); // s 仍然有效
}
```

`&s` 语法创建了一个指向 `s` 的引用，但不会转移所有权。函数参数 `&String` 表示接受一个不可变引用。当函数返回时，引用被销毁，但原始值 `s` 仍然有效。

### 可变借用（Mutable Borrowing）

当你需要修改借用的数据时，可以使用可变引用：

```rust
fn change(some_string: &mut String) {
    some_string.push_str(", world");
}

fn main() {
    let mut s = String::from("hello");
    change(&mut s);
    println!("{}", s); // 输出: hello, world
}
```

### 借用规则

Rust 的借用检查器强制执行以下规则，这些规则在编译期就能防止数据竞争：

1. **在任意给定时刻，只能有一个可变引用** 或 **任意数量的不可变引用**
2. **引用必须始终有效**（不能指向已释放的内存）

```rust
fn main() {
    let mut s = String::from("hello");
    
    let r1 = &s; // 不可变引用
    let r2 = &s; // 另一个不可变引用（允许）
    // let r3 = &mut s; // 编译错误！不可与可变引用共存
    
    println!("{} and {}", r1, r2);
    
    // r1 和 r2 在此之后不再使用
    let r3 = &mut s; // 现在可以创建可变引用
    r3.push_str("!");
    println!("{}", r3);
}
```

Rust 编译器使用**非词法生命周期**（Non-Lexical Lifetimes, NLL）来精确追踪引用的使用范围，允许在上面的例子中，当 `r1` 和 `r2` 不再被使用时创建 `r3`。

## 生命周期（Lifetime）：引用的有效期

生命周期是 Rust 编译器用来确保引用始终指向有效数据的机制。它们描述了引用在程序执行期间的有效范围。

### 显式生命周期注解

当函数返回引用时，编译器需要知道返回的引用与哪个参数的生命周期相关联：

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}

fn main() {
    let string1 = String::from("long string is long");
    let result;
    {
        let string2 = String::from("xyz");
        result = longest(string1.as_str(), string2.as_str());
        println!("最长字符串是 {}", result); // 正确
    } // string2 在此被释放
    // println!("最长字符串是 {}", result); // 编译错误！result 可能引用 string2
}
```

`'a` 是一个生命周期参数，表示 `x`、`y` 和返回值的生命周期至少是 `'a`。编译器确保返回的引用不会比任何一个输入引用活得更久。

### 结构体中的生命周期

当结构体包含引用字段时，必须为每个引用标注生命周期：

```rust
struct ImportantExcerpt<'a> {
    part: &'a str,
}

impl<'a> ImportantExcerpt<'a> {
    fn level(&self) -> i32 {
        3
    }
    
    fn announce_and_return_part(&self, announcement: &str) -> &str {
        println!("注意：{}", announcement);
        self.part
    }
}

fn main() {
    let novel = String::from("Call me Ishmael. Some years ago...");
    let first_sentence = novel.split('.').next().expect("找不到 '.'");
    let i = ImportantExcerpt {
        part: first_sentence,
    };
    // i 不能比 novel 活得更久
}
```

### 生命周期省略规则

为了减轻开发者的负担，Rust 编译器在某些情况下可以自动推断生命周期：

1. **每个引用参数都有自己的生命周期参数**
2. **如果只有一个输入生命周期参数，该生命周期被赋给所有输出生命周期参数**
3. **如果有多个输入生命周期参数，但其中一个是 `&self` 或 `&mut self`，那么 `self` 的生命周期被赋给所有输出生命周期参数**

```rust
// 以下两个函数签名等价（得益于生命周期省略）
fn first_word(s: &str) -> &str { }
fn first_word<'a>(s: &'a str) -> &'a str { }
```

## 'static 生命周期

`'static` 是 Rust 中最长的生命周期，它表示引用可以存活于整个程序运行期间。字符串字面量默认具有 `'static` 生命周期：

```rust
let s: &'static str = "我拥有 'static 生命周期";
```

需要注意的是，`'static` 并不等同于全局变量或永不释放的内存。它只是表示该引用在程序运行期间始终有效。

## 生命周期与闭包

闭包可以捕获其环境中的变量。Rust 提供了三种捕获方式，对应不同的借用语义：

```rust
fn make_adder(x: i32) -> impl Fn(i32) -> i32 {
    move |y| x + y // move 关键字强制闭包获取所有权
}

fn main() {
    let add_five = make_adder(5);
    println!("{}", add_five(3)); // 输出 8
    println!("{}", add_five(10)); // 输出 15
}
```

使用 `move` 关键字时，闭包会获取捕获变量的所有权，这在需要将闭包返回或传递给其他线程时特别有用。

## 常见陷阱与最佳实践

### 1. 悬垂引用（Dangling References）

Rust 编译器会阻止悬垂引用的产生：

```rust
fn dangle() -> &String { // 编译错误！
    let s = String::from("hello");
    &s // s 在函数结束时被释放
} // 返回的引用指向已释放的内存
```

正确的做法：返回 `String` 本身（转移所有权）或使用 `String` 的切片。

### 2. 内部可变性（Interior Mutability）

有时需要在拥有不可变引用的情况下修改数据。Rust 提供了 `RefCell<T>` 和 `Cell<T>` 等类型来实现**内部可变性**：

```rust
use std::cell::RefCell;

fn main() {
    let data = RefCell::new(5);
    
    {
        let mut borrow = data.borrow_mut();
        *borrow += 1;
    } // 可变借用在此结束
    
    println!("{}", data.borrow()); // 输出 6
}
```

### 3. 智能指针与所有权

Rust 标准库提供了多种智能指针来辅助内存管理：

| 智能指针 | 用途 | 所有权模型 |
|----------|------|-----------|
| `Box<T>` | 堆分配 | 独占所有权 |
| `Rc<T>` | 引用计数 | 共享所有权（单线程） |
| `Arc<T>` | 原子引用计数 | 共享所有权（多线程） |
| `RefCell<T>` | 运行时借用检查 | 内部可变性 |

## 总结

Rust 的所有权系统虽然初学者可能会觉得复杂，但一旦掌握，就能编写出既高效又安全的代码。核心理念可以概括为：**编译器通过所有权、借用和生命周期规则，在编译期验证所有内存访问的安全性**。

| 概念 | 核心作用 | 使用场景 |
|------|---------|---------|
| 所有权 | 管理值的生命周期 | 所有堆分配的数据 |
| 不可变借用 | 安全地读取数据 | 函数参数、迭代器 |
| 可变借用 | 安全地修改数据 | 状态修改、缓冲区操作 |
| 生命周期 | 验证引用的有效性 | 返回引用、结构体包含引用 |
| Copy trait | 自动按位复制 | 栈上的简单类型 |

Rust 的设计哲学是：**将内存安全责任从程序员转移到编译器**。虽然这意味着需要花更多时间学习与编译器"沟通"，但换来的是零成本抽象的内存安全和无与伦比的运行时性能。随着 Rust 在 Linux 内核、Web 浏览器（Firefox）、云计算（AWS、Azure）等领域的广泛应用，掌握 Rust 的所有权系统已经成为现代系统程序员的核心技能之一。

---

*本文首发于 Yggdrasil 博客，转载请注明出处。*
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '7 days',
    NOW() - INTERVAL '7 days',
    NOW() - INTERVAL '7 days'
),
(
    1,
    'Python 装饰器完全指南：从入门到精通',
    'python-decorators-guide',
    '从基础到高级，全面掌握 Python 装饰器的使用方法，包括函数装饰器、类装饰器、参数化装饰器，以及 functools.wraps、lru_cache 等内置装饰器的实战应用。',
    $doc$
# Python 装饰器完全指南：从入门到精通

装饰器（Decorator）是 Python 中最优雅、最强大的特性之一。它本质上是一种**高阶函数**（Higher-Order Function），允许你在不修改原函数源代码的前提下，为函数添加额外的功能。装饰器广泛应用于日志记录、权限验证、缓存、性能监控等场景，是 Python 元编程的核心工具。

本文将从基础概念出发，逐步深入到高级用法，帮助你全面掌握 Python 装饰器。

## 什么是装饰器？

在 Python 中，函数是一等公民（First-Class Citizen），这意味着函数可以像普通变量一样被赋值、传递和返回。装饰器正是利用了这一特性：

> **装饰器是一个接受函数作为参数并返回函数的可调用对象。**

最简单的装饰器形式如下：

```python
def my_decorator(func):
    def wrapper():
        print("🚀 函数调用前")
        func()
        print("✅ 函数调用后")
    return wrapper

@my_decorator
def say_hello():
    print("Hello, World!")

say_hello()
```

输出结果：
```
🚀 函数调用前
Hello, World!
✅ 函数调用后
```

`@my_decorator` 语法糖等价于 `say_hello = my_decorator(say_hello)`，它将 `say_hello` 函数替换为了 `wrapper` 函数。

## 处理带参数的函数

上面的装饰器只能装饰无参数的函数。为了让装饰器更通用，需要使用 `*args` 和 `**kwargs`：

```python
def greeting_decorator(func):
    def wrapper(*args, **kwargs):
        print(f"🎯 正在调用: {func.__name__}")
        result = func(*args, **kwargs)
        print(f"✨ {func.__name__} 调用完成")
        return result
    return wrapper

@greeting_decorator
def greet(name, greeting="Hello"):
    return f"{greeting}, {name}!"

print(greet("Alice"))
print(greet("Bob", greeting="Hi"))
```

输出：
```
🎯 正在调用: greet
✨ greet 调用完成
Hello, Alice!
🎯 正在调用: greet
✨ greet 调用完成
Hi, Bob!
```

## 使用 functools.wraps 保留元数据

使用装饰器后，原函数的元数据（如函数名、文档字符串）会丢失：

```python
print(say_hello.__name__)  # 输出: wrapper，而不是 say_hello
```

为了解决这个问题，Python 标准库提供了 `functools.wraps`：

```python
import functools

def my_decorator(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        """wrapper 的文档"""
        return func(*args, **kwargs)
    return wrapper

@my_decorator
def say_hello():
    """打招呼函数"""
    print("Hello!")

print(say_hello.__name__)  # 输出: say_hello
print(say_hello.__doc__)   # 输出: 打招呼函数
```

**最佳实践**：编写装饰器时务必使用 `@functools.wraps(func)`，这是专业 Python 代码的标志。

## 参数化装饰器

有时装饰器本身需要接收参数。实现参数化装饰器需要嵌套三层函数：

```python
def repeat(num=2):
    """重复执行函数 num 次"""
    def decorator(func):
        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            results = []
            for i in range(num):
                print(f"第 {i + 1} 次执行...")
                result = func(*args, **kwargs)
                results.append(result)
            return results
        return wrapper
    return decorator

@repeat(num=3)
def greet(name):
    return f"Hello, {name}!"

results = greet("Alice")
print(results)
```

输出：
```
第 1 次执行...
第 2 次执行...
第 3 次执行...
['Hello, Alice!', 'Hello, Alice!', 'Hello, Alice!']
```

执行流程：`repeat(num=3)` → 返回 `decorator` → `decorator(greet)` → 返回 `wrapper` → `greet` 被替换为 `wrapper`。

## 类装饰器

除了函数装饰器，Python 还支持**类装饰器**。类装饰器通常用于：

1. 为类添加新方法或属性
2. 修改类的行为
3. 注册类到某个注册表中

### 基础类装饰器

```python
class CountCalls:
    """统计函数被调用的次数"""
    
    def __init__(self, func):
        functools.update_wrapper(self, func)
        self.func = func
        self.num_calls = 0

    def __call__(self, *args, **kwargs):
        self.num_calls += 1
        print(f"[{self.num_calls}] {self.func.__name__} 被调用")
        return self.func(*args, **kwargs)

@CountCalls
def say_whee():
    print("Whee! 🎉")

say_whee()
say_whee()
say_whee()
print(f"总共调用了 {say_whee.num_calls} 次")
```

输出：
```
[1] say_whee 被调用
Whee! 🎉
[2] say_whee 被调用
Whee! 🎉
[3] say_whee 被调用
Whee! 🎉
总共调用了 3 次
```

### 使用类实现带状态装饰器

类装饰器特别适合需要维护状态的场景：

```python
class Cache:
    """简单的缓存装饰器"""
    
    def __init__(self, func):
        functools.update_wrapper(self, func)
        self.func = func
        self.cache = {}

    def __call__(self, *args):
        if args in self.cache:
            print(f"📦 缓存命中: {args}")
            return self.cache[args]
        
        print(f"🔍 计算中: {args}")
        result = self.func(*args)
        self.cache[args] = result
        return result

@Cache
def fibonacci(n):
    if n < 2:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

print(f"fib(5) = {fibonacci(5)}")
print(f"fib(5) = {fibonacci(5)}")  # 第二次直接返回缓存
```

## 内置装饰器实战

Python 标准库和 functools 模块提供了多个实用的内置装饰器。

### @property：将方法变为属性

```python
class Circle:
    def __init__(self, radius):
        self._radius = radius

    @property
    def radius(self):
        """获取半径"""
        return self._radius

    @radius.setter
    def radius(self, value):
        """设置半径，支持验证"""
        if value < 0:
            raise ValueError("半径不能为负数")
        self._radius = value

    @property
    def area(self):
        """计算面积"""
        return 3.14159 * self._radius ** 2

c = Circle(5)
print(f"半径: {c.radius}")
print(f"面积: {c.area:.2f}")
c.radius = 10
print(f"新面积: {c.area:.2f}")
```

### @staticmethod 和 @classmethod

```python
class DateUtil:
    @staticmethod
    def is_leap_year(year):
        """判断闰年"""
        return year % 4 == 0 and (year % 100 != 0 or year % 400 == 0)

    @classmethod
    def from_string(cls, date_str):
        """从字符串创建实例"""
        year, month, day = map(int, date_str.split('-'))
        return cls(year, month, day)

print(DateUtil.is_leap_year(2024))  # True
```

### @functools.lru_cache：自动缓存

```python
from functools import lru_cache

@lru_cache(maxsize=128)
def fibonacci(n):
    """带缓存的斐波那契数列"""
    if n < 2:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

# 计算第 100 项
print(f"fib(100) = {fibonacci(100)}")
print(f"缓存信息: {fibonacci.cache_info()}")
```

### @functools.singledispatch：函数重载

```python
from functools import singledispatch

@singledispatch
def process(arg):
    """默认实现"""
    raise NotImplementedError(f"不支持类型: {type(arg)}")

@process.register(int)
def _(arg):
    return f"整数: {arg * 2}"

@process.register(str)
def _(arg):
    return f"字符串: {arg.upper()}"

@process.register(list)
def _(arg):
    return f"列表: {len(arg)} 个元素"

print(process(42))           # 整数: 84
print(process("hello"))      # 字符串: HELLO
print(process([1, 2, 3]))    # 列表: 3 个元素
```

## 装饰器组合与顺序

多个装饰器可以叠加使用，执行顺序为**从下往上**：

```python
@decorator_a
@decorator_b
def my_func():
    pass

# 等价于: my_func = decorator_a(decorator_b(my_func))
```

## 实际应用场景

| 场景 | 装饰器实现 | 说明 |
|------|-----------|------|
| 日志记录 | `@log_call` | 记录函数调用参数和返回值 |
| 权限验证 | `@require_login` | 检查用户是否已登录 |
| 性能计时 | `@timer` | 测量函数执行时间 |
| 重试机制 | `@retry(max_attempts=3)` | 失败时自动重试 |
| 限流控制 | `@rate_limit(100)` | 限制每秒调用次数 |
| 输入验证 | `@validate_schema` | 验证函数参数格式 |

## 总结

装饰器是 Python 中最具表现力的特性之一，它让代码更加简洁、可复用和符合 DRY 原则。掌握装饰器需要理解：

1. **函数是一等公民**：函数可以作为参数传递、作为返回值返回
2. **闭包**：装饰器内部函数可以访问外部函数的变量
3. **@语法糖**：`@decorator` 等价于 `func = decorator(func)`
4. **functools.wraps**：保留原函数的元数据
5. **嵌套三层**：实现参数化装饰器需要额外的一层函数

> "Python 的装饰器让代码像诗歌一样优美，但过度使用会让代码变得晦涩难懂。" —— 遵循**显式优于隐式**的原则，在合适的场景使用装饰器，避免过度工程化。

---

*本文首发于 Yggdrasil 博客*
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '6 days',
    NOW() - INTERVAL '6 days',
    NOW() - INTERVAL '6 days'
),
(
    1,
    'JavaScript 异步编程完全指南：从回调地狱到 async/await',
    'js-async-programming',
    '深入理解 JavaScript 的异步编程模型，从回调函数到 Promise，再到 async/await 的演变历程，以及事件循环的底层原理。',
    $doc$
# JavaScript 异步编程完全指南：从回调地狱到 async/await

JavaScript 是一门单线程语言，这意味着它一次只能执行一个任务。然而，现代 Web 应用需要处理大量 I/O 操作（如网络请求、文件读写、定时器等），如果采用同步方式处理，整个程序会被阻塞，用户体验极差。为了解决这一问题，JavaScript 采用了**事件驱动、非阻塞 I/O** 的编程模型，并通过**事件循环**（Event Loop）机制实现了异步编程。

本文将带你回顾 JavaScript 异步编程的完整演进历程，从最早的回调函数到现代的 async/await，帮助你建立对 JavaScript 并发模型的系统理解。

## 回调函数：异步编程的起点

在 JavaScript 早期，异步操作主要通过回调函数（Callback）来实现。回调函数是一个被作为参数传递给另一个函数的函数，当异步操作完成时被调用：

```javascript
function fetchData(callback) {
    setTimeout(() => {
        callback("数据加载完成");
    }, 1000);
}

fetchData((data) => {
    console.log(data);
});
```

回调函数简单直观，但存在两个主要问题：

### 1. 回调地狱（Callback Hell）

当多个异步操作需要按顺序执行时，代码会层层嵌套，形成"金字塔型"结构：

```javascript
getUserData(userId, (user) => {
    getOrders(user.id, (orders) => {
        getOrderDetails(orders[0].id, (details) => {
            getProductInfo(details.productId, (product) => {
                console.log(product);
            });
        });
    });
});
```

这种代码难以阅读、维护和调试。每一层嵌套都增加了认知负担，错误处理也变得异常复杂。

### 2. 错误处理困难

回调函数通常采用"错误优先"的约定（Error-First Callback）：

```javascript
fs.readFile('file.txt', (err, data) => {
    if (err) {
        console.error('读取失败:', err);
        return;
    }
    console.log('文件内容:', data);
});
```

虽然这种约定统一了错误处理方式，但在多层嵌套时，错误处理代码会散落在各个层级，导致大量重复和遗漏。

## Promise：优雅的异步解决方案

ES6（2015）引入了 **Promise**，它是对异步操作的一种抽象表示，代表一个尚未完成但预期将来会完成的操作。

### Promise 的基本用法

Promise 有三种状态：
- **Pending（待定）**：初始状态，操作尚未完成
- **Fulfilled（已完成）**：操作成功完成
- **Rejected（已拒绝）**：操作失败

```javascript
const fetchData = () => {
    return new Promise((resolve, reject) => {
        setTimeout(() => {
            const success = true;
            if (success) {
                resolve("✅ 数据加载成功");
            } else {
                reject("❌ 数据加载失败");
            }
        }, 1000);
    });
};

fetchData()
    .then(data => {
        console.log(data);
        return "处理后的数据";
    })
    .then(processedData => {
        console.log(processedData);
    })
    .catch(err => {
        console.error("错误:", err);
    })
    .finally(() => {
        console.log("无论成功还是失败，都会执行");
    });
```

### Promise 链式调用

Promise 的最大优势在于支持**链式调用**，解决了回调地狱问题：

```javascript
getUserData(userId)
    .then(user => getOrders(user.id))
    .then(orders => getOrderDetails(orders[0].id))
    .then(details => getProductInfo(details.productId))
    .then(product => console.log(product))
    .catch(err => console.error("出错了:", err));
```

每个 `.then()` 接收前一个 Promise 的返回值，并返回一个新的 Promise，形成清晰的线性流程。

### Promise.all 和 Promise.race

Promise 还提供了组合多个异步操作的方法：

```javascript
// 并行执行多个 Promise，全部完成后返回
const promises = [
    fetchUserData(),
    fetchUserSettings(),
    fetchUserNotifications()
];

Promise.all(promises)
    .then(([user, settings, notifications]) => {
        console.log("所有数据加载完成");
    })
    .catch(err => console.error("任一失败:", err));

// 返回最快完成的 Promise
Promise.race([
    fetchData(),
    new Promise((_, reject) => 
        setTimeout(() => reject("超时"), 5000)
    )
])
    .then(data => console.log(data))
    .catch(err => console.error(err));
```

## Async/Await：异步代码的同步写法

ES2017 引入了 **async/await**，它是 Promise 的语法糖，让异步代码看起来像同步代码一样直观：

```javascript
async function loadUserData(userId) {
    try {
        const user = await getUserData(userId);
        const orders = await getOrders(user.id);
        const details = await getOrderDetails(orders[0].id);
        const product = await getProductInfo(details.productId);
        
        return product;
    } catch (err) {
        console.error("加载用户数据失败:", err);
        throw err;
    } finally {
        console.log("数据加载流程结束");
    }
}

// 调用 async 函数
loadUserData(123)
    .then(product => console.log(product))
    .catch(err => console.error(err));
```

### async/await 的优势

1. **代码更扁平**：消除了 Promise 链的嵌套
2. **错误处理更自然**：使用 `try/catch`，符合同步代码的习惯
3. **调试更方便**：可以在 await 处设置断点
4. **条件语句更直观**：

```javascript
async function fetchData(shouldFetchDetails) {
    const user = await getUserData();
    
    // 条件执行异步操作
    if (shouldFetchDetails) {
        const details = await getDetails(user.id);
        return { user, details };
    }
    
    return { user };
}
```

### 并行执行 await

默认情况下，await 会按顺序执行，但可以通过 `Promise.all` 实现并行：

```javascript
async function loadDashboard() {
    // 串行执行（较慢）
    // const user = await getUserData();
    // const posts = await getPosts();
    // const notifications = await getNotifications();
    
    // 并行执行（更快）
    const [user, posts, notifications] = await Promise.all([
        getUserData(),
        getPosts(),
        getNotifications()
    ]);
    
    return { user, posts, notifications };
}
```

## 事件循环：JavaScript 的并发心脏

要真正理解 JavaScript 的异步机制，必须深入理解**事件循环**（Event Loop）。事件循环是 JavaScript 运行时（如浏览器或 Node.js）的核心机制，负责协调同步代码和异步回调的执行。

### 调用栈（Call Stack）

调用栈是一个后进先出（LIFO）的数据结构，用于追踪程序的执行位置。当函数被调用时，它被压入栈顶；当函数返回时，它从栈顶弹出。

### 任务队列（Task Queue）

异步操作完成后，其回调函数不会立即执行，而是被放入**任务队列**中等待。事件循环不断地检查调用栈是否为空，如果为空，则将任务队列中的第一个任务推入调用栈执行。

### 宏任务与微任务

JavaScript 的任务队列分为两类：

| 类型 | 包含 | 优先级 |
|------|------|--------|
| **宏任务（Macrotask）** | `setTimeout`、`setInterval`、I/O 操作、UI 渲染 | 低 |
| **微任务（Microtask）** | `Promise.then()`、`Promise.catch()`、`MutationObserver`、`queueMicrotask()` | 高 |

**执行规则**：
1. 执行当前宏任务（同步代码）
2. 执行所有微任务（清空微任务队列）
3. 渲染 UI（如果需要）
4. 从宏任务队列取出下一个任务，重复步骤 1

### 经典示例分析

```javascript
console.log("1️⃣ 同步代码");

setTimeout(() => {
    console.log("2️⃣ setTimeout（宏任务）");
}, 0);

Promise.resolve().then(() => {
    console.log("3️⃣ Promise.then（微任务）");
});

console.log("4️⃣ 同步代码结束");
```

输出顺序：
```
1️⃣ 同步代码
4️⃣ 同步代码结束
3️⃣ Promise.then（微任务）
2️⃣ setTimeout（宏任务）
```

**执行流程解析**：
1. `console.log("1️⃣")` 立即执行
2. `setTimeout` 的回调被放入宏任务队列
3. `Promise.resolve().then()` 的回调被放入微任务队列
4. `console.log("4️⃣")` 立即执行
5. 同步代码执行完毕，检查微任务队列，执行 Promise 回调
6. 微任务队列为空，从宏任务队列取出 setTimeout 回调执行

### 更复杂的示例

```javascript
console.log("Start");

setTimeout(() => {
    console.log("Timeout 1");
    Promise.resolve().then(() => {
        console.log("Promise inside timeout");
    });
}, 0);

setTimeout(() => {
    console.log("Timeout 2");
}, 0);

Promise.resolve().then(() => {
    console.log("Promise 1");
}).then(() => {
    console.log("Promise 2");
});

console.log("End");
```

输出：
```
Start
End
Promise 1
Promise 2
Timeout 1
Promise inside timeout
Timeout 2
```

## 现代异步模式

### 1. for-await-of 循环

ES2018 引入了异步迭代器，可以优雅地遍历异步数据源：

```javascript
async function* fetchPages() {
    for (let i = 1; i <= 3; i++) {
        yield await fetch(`/api/page/${i}`).then(r => r.json());
    }
}

(async () => {
    for await (const page of fetchPages()) {
        console.log(page);
    }
})();
```

### 2. AbortController 取消请求

```javascript
const controller = new AbortController();

fetch('/api/data', { signal: controller.signal })
    .then(response => response.json())
    .then(data => console.log(data))
    .catch(err => {
        if (err.name === 'AbortError') {
            console.log('请求已被取消');
        }
    });

// 5 秒后取消请求
setTimeout(() => controller.abort(), 5000);
```

### 3. Top-level await（ES2022）

```javascript
// 模块顶层直接使用 await
const data = await fetch('/api/config').then(r => r.json());
export { data };
```

## 总结

JavaScript 的异步编程经历了从回调到 Promise，再到 async/await 的演变，每一步都让代码更加优雅和易于维护。

| 机制 | 优点 | 缺点 | 适用场景 |
|------|------|------|---------|
| 回调函数 | 简单直观 | 回调地狱、错误处理困难 | 简单的异步操作 |
| Promise | 链式调用、组合方便 | 仍有一定嵌套 | 多步骤异步流程 |
| async/await | 同步写法、易调试 | 需要理解底层 Promise | 复杂异步逻辑 |

**关键要点**：
- 微任务优先级高于宏任务
- `await` 后面的代码会被放入微任务队列
- 使用 `Promise.all` 并行执行多个异步操作
- 始终使用 `try/catch` 处理 async/await 中的错误

> 深入理解事件循环是掌握 JavaScript 异步编程的关键。推荐阅读：[MDN - 使用 Promises](https://developer.mozilla.org/zh-CN/docs/Web/JavaScript/Guide/Using_promises)

---

*本文首发于 Yggdrasil 博客*
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '5 days',
    NOW() - INTERVAL '5 days',
    NOW() - INTERVAL '5 days'
),
(
    1,
    'Go 并发模式完全指南：Goroutine、Channel 与 Select',
    'go-concurrency-patterns',
    'Go 语言以简洁的并发模型著称，本文详细介绍 goroutine、channel、select 和各种并发设计模式，帮助你写出高性能的并发程序。',
    $doc$
# Go 并发模式完全指南：Goroutine、Channel 与 Select

Go 语言自 2009 年发布以来，凭借其**简洁、高效、原生支持并发**的特性，迅速成为云原生时代的首选语言之一。Docker、Kubernetes、Prometheus 等知名项目均使用 Go 编写。Go 语言最引人注目的特性之一，就是其内置的轻量级线程 **goroutine** 和通信原语 **channel**，它们让并发编程变得简单而优雅。

与操作系统线程（通常占用数 MB 内存）不同，goroutine 的初始栈仅有 **2KB**，并且可以根据需要动态增长和收缩。这意味着在单个 Go 程序中可以轻松创建数十万个 goroutine，而不会耗尽系统资源。

本文将系统介绍 Go 的并发原语和常见并发模式，帮助你写出高效、健壮的并发程序。

## Goroutine：轻量级并发单元

在 Go 中，启动一个并发任务只需在函数调用前加上 `go` 关键字：

```go
package main

import (
    "fmt"
    "time"
)

func say(s string, times int) {
    for i := 0; i < times; i++ {
        time.Sleep(100 * time.Millisecond)
        fmt.Printf("%s (第 %d 次)\\n", s, i+1)
    }
}

func main() {
    // 启动两个 goroutine 并发执行
    go say("🌏 world", 3)
    go say("👋 hello", 3)
    
    // 主 goroutine 等待一段时间，让子 goroutine 完成
    time.Sleep(1 * time.Second)
    fmt.Println("主程序结束")
}
```

输出（顺序可能不同）：
```
👋 hello (第 1 次)
🌏 world (第 1 次)
🌏 world (第 2 次)
👋 hello (第 2 次)
👋 hello (第 3 次)
🌏 world (第 3 次)
主程序结束
```

### 使用 WaitGroup 等待 goroutine 完成

上面的示例使用了 `time.Sleep` 来等待 goroutine，这种方式不够精确。Go 提供了 `sync.WaitGroup` 来优雅地等待一组 goroutine 完成：

```go
package main

import (
    "fmt"
    "sync"
    "time"
)

func worker(id int, wg *sync.WaitGroup) {
    defer wg.Done() // 完成时减少计数器
    
    fmt.Printf("🚀 Worker %d 开始工作\\n", id)
    time.Sleep(time.Second)
    fmt.Printf("✅ Worker %d 完成工作\\n", id)
}

func main() {
    var wg sync.WaitGroup
    
    for i := 1; i <= 3; i++ {
        wg.Add(1) // 增加计数器
        go worker(i, &wg)
    }
    
    wg.Wait() // 等待所有 goroutine 完成
    fmt.Println("所有工作已完成")
}
```

## Channel：goroutine 之间的通信桥梁

Go 语言的设计哲学是：**不要通过共享内存来通信，而要通过通信来共享内存**（Do not communicate by sharing memory; instead, share memory by communicating.）。Channel 正是这一哲学的核心实现。

Channel 是一种类型安全的队列，用于在 goroutine 之间传递数据。

### 创建 Channel

```go
// 无缓冲 channel（同步通信）
ch := make(chan int)

// 有缓冲 channel（异步通信，容量为 5）
ch := make(chan int, 5)
```

### 无缓冲 Channel

无缓冲 channel 在发送和接收时会阻塞，直到对方准备好：

```go
package main

import "fmt"

func main() {
    ch := make(chan string)
    
    go func() {
        fmt.Println("📤 发送消息...")
        ch <- "Hello from goroutine!"
        fmt.Println("📤 发送完成")
    }()
    
    msg := <-ch // 阻塞等待接收
    fmt.Println("📨 收到:", msg)
}
```

### 有缓冲 Channel

有缓冲 channel 允许在阻塞前发送指定数量的数据：

```go
package main

import "fmt"

func main() {
    ch := make(chan int, 3)
    
    // 发送数据（不会阻塞，因为缓冲区未满）
    ch <- 1
    ch <- 2
    ch <- 3
    
    // ch <- 4 // 如果取消注释，此行会阻塞，因为缓冲区已满
    
    // 接收数据
    fmt.Println(<-ch) // 1
    fmt.Println(<-ch) // 2
    fmt.Println(<-ch) // 3
}
```

### 关闭 Channel

当发送方不再发送数据时，应该关闭 channel：

```go
package main

import "fmt"

func producer(ch chan<- int) {
    for i := 0; i < 5; i++ {
        ch <- i
    }
    close(ch) // 关闭 channel
}

func main() {
    ch := make(chan int)
    go producer(ch)
    
    // 使用 range 遍历 channel，自动检测关闭
    for value := range ch {
        fmt.Printf("收到: %d\\n", value)
    }
    fmt.Println("Channel 已关闭")
}
```

### 单向 Channel

Go 支持单向 channel 类型，用于限制 channel 的使用方向：

```go
func producer(ch chan<- int) { // 只发送
    ch <- 42
}

func consumer(ch <-chan int) { // 只接收
    fmt.Println(<-ch)
}
```

## Select：多路复用

`select` 语句是 Go 并发编程的瑞士军刀，它允许你同时等待多个 channel 操作，类似于网络编程中的 `select()` 系统调用：

```go
package main

import (
    "fmt"
    "time"
)

func main() {
    ch1 := make(chan string)
    ch2 := make(chan string)
    
    go func() {
        time.Sleep(1 * time.Second)
        ch1 <- "来自 channel 1"
    }()
    
    go func() {
        time.Sleep(2 * time.Second)
        ch2 <- "来自 channel 2"
    }()
    
    // 等待两个 channel 中的任意一个
    select {
    case msg1 := <-ch1:
        fmt.Println(msg1)
    case msg2 := <-ch2:
        fmt.Println(msg2)
    }
}
```

### 超时处理

```go
select {
case res := <-c1:
    fmt.Println("结果:", res)
case <-time.After(1 * time.Second):
    fmt.Println("⏰ 超时！")
}
```

### 非阻塞操作

```go
select {
case ch <- value:
    fmt.Println("发送成功")
default:
    fmt.Println("channel 已满，跳过")
}
```

### 随机选择

当多个 case 同时就绪时，select 会**随机**选择一个执行：

```go
ch := make(chan int, 2)
ch <- 1
ch <- 2

select {
case <-ch:
    fmt.Println("收到数据 1")
case <-ch:
    fmt.Println("收到数据 2")
}
// 随机输出其中一个
```

## 常见并发模式

### 1. 生产者-消费者模式

```go
package main

import (
    "fmt"
    "time"
)

func producer(id int, ch chan<- int) {
    for i := 0; i < 3; i++ {
        item := id*10 + i
        fmt.Printf("🏭 生产者 %d 生产: %d\\n", id, item)
        ch <- item
        time.Sleep(100 * time.Millisecond)
    }
}

func consumer(id int, ch <-chan int) {
    for item := range ch {
        fmt.Printf("🛒 消费者 %d 消费: %d\\n", id, item)
        time.Sleep(200 * time.Millisecond)
    }
}

func main() {
    ch := make(chan int, 5)
    
    // 启动 2 个生产者
    for i := 1; i <= 2; i++ {
        go producer(i, ch)
    }
    
    // 启动 3 个消费者
    for i := 1; i <= 3; i++ {
        go consumer(i, ch)
    }
    
    time.Sleep(3 * time.Second)
    close(ch)
    time.Sleep(1 * time.Second)
}
```

### 2. Worker Pool（工作池）

```go
package main

import (
    "fmt"
    "sync"
    "time"
)

func worker(id int, jobs <-chan int, results chan<- int, wg *sync.WaitGroup) {
    defer wg.Done()
    for job := range jobs {
        fmt.Printf("👷 Worker %d 处理任务 %d\\n", id, job)
        time.Sleep(time.Second) // 模拟工作
        results <- job * 2
    }
}

func main() {
    const numJobs = 10
    const numWorkers = 3
    
    jobs := make(chan int, numJobs)
    results := make(chan int, numJobs)
    
    var wg sync.WaitGroup
    for w := 1; w <= numWorkers; w++ {
        wg.Add(1)
        go worker(w, jobs, results, &wg)
    }
    
    for j := 1; j <= numJobs; j++ {
        jobs <- j
    }
    close(jobs)
    
    wg.Wait()
    close(results)
    
    for result := range results {
        fmt.Printf("📊 结果: %d\\n", result)
    }
}
```

### 3. Pipeline（管道）

```go
package main

import "fmt"

func gen(nums ...int) <-chan int {
    out := make(chan int)
    go func() {
        for _, n := range nums {
            out <- n
        }
        close(out)
    }()
    return out
}

func sq(in <-chan int) <-chan int {
    out := make(chan int)
    go func() {
        for n := range in {
            out <- n * n
        }
        close(out)
    }()
    return out
}

func main() {
    // 设置 pipeline: gen -> sq
    for result := range sq(sq(gen(2, 3, 4))) {
        fmt.Println(result) // 16, 81, 256 (先平方再平方)
    }
}
```

### 4. Fan-out/Fan-in（扇出/扇入）

```go
func merge(channels ...<-chan int) <-chan int {
    var wg sync.WaitGroup
    out := make(chan int)
    
    output := func(c <-chan int) {
        defer wg.Done()
        for n := range c {
            out <- n
        }
    }
    
    wg.Add(len(channels))
    for _, c := range channels {
        go output(c)
    }
    
    go func() {
        wg.Wait()
        close(out)
    }()
    
    return out
}
```

## Context：请求级控制

Go 1.7 引入了 `context` 包，用于在 goroutine 之间传递截止时间、取消信号和请求范围的值：

```go
package main

import (
    "context"
    "fmt"
    "time"
)

func slowOperation(ctx context.Context) (string, error) {
    select {
    case <-time.After(2 * time.Second):
        return "✅ 操作完成", nil
    case <-ctx.Done():
        return "", ctx.Err()
    }
}

func main() {
    // 设置 1 秒超时
    ctx, cancel := context.WithTimeout(context.Background(), 1*time.Second)
    defer cancel()
    
    result, err := slowOperation(ctx)
    if err != nil {
        fmt.Println("❌ 错误:", err) // context deadline exceeded
        return
    }
    fmt.Println(result)
}
```

### Context 链式传递

```go
func main() {
    ctx := context.Background()
    
    // 添加超时
    ctx, cancel := context.WithTimeout(ctx, 5*time.Second)
    defer cancel()
    
    // 添加取消信号
    ctx, cancel = context.WithCancel(ctx)
    defer cancel()
    
    // 添加请求 ID
    ctx = context.WithValue(ctx, "requestID", "abc-123")
    
    processRequest(ctx)
}
```

## 并发模式对比

| 模式 | 描述 | 使用场景 | 关键原语 |
|------|------|---------|---------|
| 生产者-消费者 | 通过 channel 传递数据 | 任务队列、消息处理 | `chan`, `range` |
| Worker Pool | 固定数量的 worker 处理任务 | CPU 密集型任务 | `sync.WaitGroup` |
| Pipeline | 多个 stage 串联处理 | 数据流处理 | `chan` 返回 |
| Fan-out | 多路分发 | 并行处理 | 多个 goroutine |
| Fan-in | 多路合并 | 聚合结果 | `select`, `sync.WaitGroup` |
| 超时控制 | 限制操作时间 | 网络请求、外部调用 | `context`, `time.After` |

## 总结

Go 的并发模型以其简洁和高效著称。核心要点：

1. **Goroutine** 是轻量级线程，通过 `go` 关键字启动
2. **Channel** 是 goroutine 之间的通信方式，而不是共享内存
3. **Select** 实现多路复用和超时控制
4. **Context** 管理请求级的生命周期
5. **Sync 包** 提供了锁、WaitGroup、Once、Pool 等同步原语

> **Go 并发黄金法则**：先通过 channel 思考，只有在必要时才使用共享内存和锁。

---

*本文首发于 Yggdrasil 博客*
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '4 days',
    NOW() - INTERVAL '4 days',
    NOW() - INTERVAL '4 days'
),
(
    1,
    'TypeScript 高级类型体操：从条件类型到模板字面量',
    'typescript-type-gymnastics',
    '探索 TypeScript 类型系统的极限，从条件类型到映射类型，再到模板字面量类型和类型推断，让你的代码在编译期就获得强大的类型保障。',
    $doc$
# TypeScript 高级类型体操：从条件类型到模板字面量

TypeScript 的类型系统是一门**图灵完备**的语言。这意味着，在理论上，你可以使用 TypeScript 的类型系统来实现任何可计算的程序。虽然我们不建议在真实项目中过度使用复杂的类型体操，但掌握这些高级类型技巧，可以帮助你构建更加健壮、可维护的类型定义，特别是在开发库和框架时。

本文将从基础到高级，系统介绍 TypeScript 的类型系统特性，包括条件类型、映射类型、模板字面量类型、递归类型和类型推断等。

## 为什么需要高级类型？

在日常开发中，基础类型（`string`、`number`、`boolean`）和简单的接口往往已经足够。但在以下场景中，高级类型变得至关重要：

- **开发类型安全的库**：如 Vue、React、Redux 等框架的类型定义
- **编写通用工具函数**：如 lodash 的类型定义
- **实现类型转换**：如将对象的所有属性变为可选或只读
- **约束 API 接口**：确保编译器能够推断出正确的响应类型

## 条件类型（Conditional Types）

条件类型是 TypeScript 2.8 引入的特性，它允许你根据类型关系选择不同的类型：

```typescript
type IsString<T> = T extends string ? true : false;

// 使用示例
type A = IsString<string>;  // true
type B = IsString<number>;  // false
type C = IsString<"hello">; // true（字面量类型也是 string 的子类型）
```

### extends 关键字

在条件类型中，`extends` 表示"是否是...的子类型"：

```typescript
type IsArray<T> = T extends any[] ? true : false;

type D = IsArray<number[]>; // true
type E = IsArray<string>;   // false
```

### infer 关键字：类型推断

`infer` 是 TypeScript 中最强大的关键字之一，它允许你在条件类型中"提取"类型：

```typescript
// 提取数组元素类型
type ElementType<T> = T extends (infer U)[] ? U : never;

type F = ElementType<string[]>; // string
type G = ElementType<number[]>; // number

// 提取函数返回值类型
type ReturnType<T> = T extends (...args: any[]) => infer R ? R : never;

function getUser() {
    return { id: 1, name: "Alice" };
}

type User = ReturnType<typeof getUser>; // { id: number; name: string; }

// 提取 Promise 的解析类型
type UnwrapPromise<T> = T extends Promise<infer U> ? U : T;

type H = UnwrapPromise<Promise<string>>; // string
```

### 内置条件类型

TypeScript 标准库已经提供了许多基于条件类型的工具类型：

| 类型 | 作用 | 示例 |
|------|------|------|
| `Exclude<T, U>` | 从 T 中排除 U | `Exclude<'a' \| 'b', 'a'>` → `'b'` |
| `Extract<T, U>` | 从 T 中提取 U | `Extract<'a' \| 'b', 'a' \| 'c'>` → `'a'` |
| `NonNullable<T>` | 排除 null 和 undefined | `NonNullable<string \| null>` → `string` |
| `ReturnType<T>` | 获取函数返回类型 | `ReturnType<() => number>` → `number` |
| `Parameters<T>` | 获取函数参数类型 | `Parameters<(a: string) => void>` → `[string]` |
| `InstanceType<T>` | 获取构造函数实例类型 | `InstanceType<typeof Date>` → `Date` |

## 映射类型（Mapped Types）

映射类型允许你基于已有类型创建新类型，通过遍历属性键来转换每个属性：

### 基础映射类型

```typescript
type Readonly<T> = {
    readonly [P in keyof T]: T[P];
};

type Partial<T> = {
    [P in keyof T]?: T[P];
};

type Required<T> = {
    [P in keyof T]-?: T[P]; // -? 移除可选性
};

// 使用示例
interface User {
    name: string;
    age: number;
}

type ReadonlyUser = Readonly<User>;
// { readonly name: string; readonly age: number; }

type PartialUser = Partial<User>;
// { name?: string; age?: number; }
```

### 键重映射（Key Remapping）

TypeScript 4.1 引入了 `as` 关键字，允许在映射类型中重命名键：

```typescript
// 将每个属性名添加 "get" 前缀，类型变为函数
type Getters<T> = {
    [K in keyof T as `get${Capitalize<string & K>}`]: () => T[K];
};

interface Person {
    name: string;
    age: number;
}

type PersonGetters = Getters<Person>;
// { getName: () => string; getAge: () => number; }
```

### 过滤属性

```typescript
// 只保留 string 类型的属性
type StringProperties<T> = {
    [K in keyof T as T[K] extends string ? K : never]: T[K];
};

interface User {
    name: string;
    age: number;
    email: string;
}

type StringUserProps = StringProperties<User>;
// { name: string; email: string; }
```

## 模板字面量类型（Template Literal Types）

TypeScript 4.1 引入了模板字面量类型，它允许你通过字符串字面量类型来构造新类型：

### 基础用法

```typescript
type EventName<T extends string> = `on${Capitalize<T>}`;

type ClickEvent = EventName<"click">;      // "onClick"
type HoverEvent = EventName<"mouseOver">;  // "onMouseOver"
```

### 联合类型的组合

当模板字面量类型与联合类型结合使用时，会产生**笛卡尔积**效果：

```typescript
type Horizontal = "left" | "center" | "right";
type Vertical = "top" | "center" | "bottom";

type Alignment = `${Horizontal}-${Vertical}`;
// "left-top" | "left-center" | "left-bottom" |
// "center-top" | "center-center" | "center-bottom" |
// "right-top" | "right-center" | "right-bottom"
```

### 实际应用：CSS 属性类型

```typescript
type CSSProperty = "margin" | "padding";
type CSSDirection = "top" | "right" | "bottom" | "left";

type CSSKey = `${CSSProperty}${Capitalize<CSSDirection>}` | CSSProperty;
// "margin" | "padding" | "marginTop" | "marginRight" | ...
```

## 递归类型

TypeScript 支持递归类型定义，这在处理嵌套数据结构时非常有用：

```typescript
// 深度只读类型
type DeepReadonly<T> = {
    readonly [P in keyof T]: T[P] extends object
        ? DeepReadonly<T[P]>
        : T[P];
};

interface NestedUser {
    name: string;
    address: {
        city: string;
        coordinates: {
            lat: number;
            lng: number;
        };
    };
}

type DeepReadonlyUser = DeepReadonly<NestedUser>;
// 所有嵌套属性都变为 readonly

// 深度 Partial 类型
type DeepPartial<T> = {
    [P in keyof T]?: T[P] extends object
        ? DeepPartial<T[P]>
        : T[P];
};

// 扁平化对象类型（将嵌套属性用点号连接）
type Flatten<T, Prefix = ""> = {
    [K in keyof T as K extends string
        ? Prefix extends ""
            ? K
            : `${Prefix & string}.${K}`
        : never
    ]: T[K] extends object ? Flatten<T[K], `${Prefix & string}.${K & string}`> : T[K];
};
```

## 类型守卫与类型收窄

TypeScript 的类型系统不仅在编译期工作，还能通过类型守卫在运行时收窄类型：

```typescript
// typeof 类型守卫
function processValue(value: string | number) {
    if (typeof value === "string") {
        // 此分支中 value 的类型为 string
        return value.toUpperCase();
    } else {
        // 此分支中 value 的类型为 number
        return value.toFixed(2);
    }
}

// 自定义类型守卫
type Cat = { kind: "cat"; meow: () => void };
type Dog = { kind: "dog"; bark: () => void };
type Animal = Cat | Dog;

function isCat(animal: Animal): animal is Cat {
    return animal.kind === "cat";
}

function makeSound(animal: Animal) {
    if (isCat(animal)) {
        animal.meow();
    } else {
        animal.bark();
    }
}
```

## 实用类型工具库

基于上述类型特性，你可以构建强大的类型工具库：

```typescript
// 将对象的所有嵌套属性路径提取为联合类型
type Path<T, K extends keyof T = keyof T> = K extends string
    ? T[K] extends object
        ? `${K}` | `${K}.${Path<T[K]>}`
        : `${K}`
    : never;

// 安全的深度属性访问类型
type DeepPick<T, P extends string> = P extends `${infer K}.${infer Rest}`
    ? K extends keyof T
        ? { [key in K]: DeepPick<T[K], Rest> }
        : never
    : P extends keyof T
    ? { [key in P]: T[P] }
    : never;
```

## 总结

TypeScript 的类型系统提供了丰富的工具来构建类型安全的应用程序：

| 特性 | 版本 | 作用 |
|------|------|------|
| 条件类型 | 2.8 | 基于类型关系选择类型 |
| infer | 2.8 | 在条件类型中提取类型 |
| 映射类型 | 2.1 | 遍历属性键转换类型 |
| 模板字面量 | 4.1 | 构造字符串字面量类型 |
| 键重映射 | 4.1 | 在映射中重命名属性键 |
| 递归类型 | 3.7+ | 处理嵌套数据结构 |

> ⚠️ **温馨提示**：类型体操虽有趣，但过度使用会降低代码可读性和编译性能。遵循"简单优于复杂"的原则，在合适的场景使用高级类型。对于大多数业务代码，基础类型和简单的接口已经足够。

---

*本文首发于 Yggdrasil 博客*
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '3 days',
    NOW() - INTERVAL '3 days',
    NOW() - INTERVAL '3 days'
),
(
    1,
    'Java 泛型深度解析：从基础到通配符的PECS原则',
    'java-generics-cheatsheet',
    'Java 泛型是类型安全的基石，本文深入讲解泛型类、泛型方法、通配符及其上界下界，以及PECS原则在实际编程中的应用。',
    $doc$
# Java 泛型深度解析：从基础到通配符的 PECS 原则

Java 泛型（Generics）是自 JDK 5 引入的一项重要特性，它为 Java 语言带来了**编译期类型检查**和**类型安全**的能力。在没有泛型之前，Java 集合只能存储 `Object` 类型，取出数据时需要进行强制类型转换，这不仅繁琐，而且容易在运行时抛出 `ClassCastException`。

本文将从基础概念出发，深入探讨 Java 泛型的核心机制，包括泛型类、泛型方法、通配符、类型擦除，以及实际开发中最常用的 PECS 原则。

## 泛型的基础概念

### 泛型类

泛型类允许你在定义类时使用类型参数，这些类型参数在创建实例时被具体化：

```java
// 定义泛型类
public class Box<T> {
    private T value;
    
    public void set(T value) {
        this.value = value;
    }
    
    public T get() {
        return value;
    }
    
    public static void main(String[] args) {
        // 创建 String 类型的 Box
        Box<String> stringBox = new Box<>();
        stringBox.set("Hello Generics");
        String str = stringBox.get(); // 无需类型转换
        
        // 创建 Integer 类型的 Box
        Box<Integer> intBox = new Box<>();
        intBox.set(42);
        Integer num = intBox.get();
    }
}
```

### 泛型方法

泛型方法允许你在普通类或泛型类中定义带有类型参数的方法：

```java
public class GenericMethodExample {
    
    // 泛型方法
    public <T> void printArray(T[] array) {
        for (T element : array) {
            System.out.println(element);
        }
    }
    
    // 泛型方法 with 返回值
    public <T> T getFirst(T[] array) {
        return array.length > 0 ? array[0] : null;
    }
    
    // 泛型方法 with 多个类型参数
    public <K, V> void printPair(K key, V value) {
        System.out.println("Key: " + key + ", Value: " + value);
    }
    
    public static void main(String[] args) {
        GenericMethodExample example = new GenericMethodExample();
        
        String[] names = {"Alice", "Bob", "Charlie"};
        example.printArray(names);
        
        Integer first = example.getFirst(new Integer[]{1, 2, 3});
        example.printPair("ID", 1001);
    }
}
```

### 类型参数的约束

你可以使用 `extends` 关键字对类型参数进行约束：

```java
// T 必须是 Number 的子类
public class NumberBox<T extends Number> {
    private T value;
    
    public double getDoubleValue() {
        return value.doubleValue();
    }
}

// 使用
NumberBox<Integer> intBox = new NumberBox<>(); // ✅
NumberBox<Double> doubleBox = new NumberBox<>(); // ✅
// NumberBox<String> stringBox = new NumberBox<>(); // ❌ 编译错误
```

## 通配符（Wildcards）

泛型中的通配符 `?` 表示**未知类型**，它提供了更灵活的类型兼容性。

### 无界通配符 `?`

```java
public void printList(List<?> list) {
    for (Object obj : list) {
        System.out.println(obj);
    }
}

// 可以接受任何类型的 List
printList(new ArrayList<String>());
printList(new ArrayList<Integer>());
```

### 上界通配符 `? extends T`

`? extends T` 表示**T 的子类型**，适用于读取数据的场景：

```java
// 可以接受 Number 及其子类型的 List
public double sumNumbers(List<? extends Number> numbers) {
    double sum = 0;
    for (Number num : numbers) {
        sum += num.doubleValue();
    }
    return sum;
}

// 使用
List<Integer> ints = Arrays.asList(1, 2, 3);
List<Double> doubles = Arrays.asList(1.1, 2.2, 3.3);

sumNumbers(ints);    // ✅ Integer 是 Number 的子类
sumNumbers(doubles); // ✅ Double 是 Number 的子类
```

### 下界通配符 `? super T`

`? super T` 表示**T 的父类型**，适用于写入数据的场景：

```java
// 可以接受 Integer 及其父类型的 List
public void addIntegers(List<? super Integer> list) {
    list.add(1);
    list.add(2);
    list.add(3);
}

// 使用
List<Number> numbers = new ArrayList<>();
List<Object> objects = new ArrayList<>();

addIntegers(numbers); // ✅ Number 是 Integer 的父类
addIntegers(objects); // ✅ Object 是 Integer 的父类
// addIntegers(new ArrayList<String>()); // ❌ 编译错误
```

## PECS 原则

PECS 是 Java 泛型中最重要也最实用的原则，它是 **Producer-Extends, Consumer-Super** 的缩写：

| 原则 | 含义 | 使用场景 |
|------|------|---------|
| **Producer-Extends** | 如果数据从集合中产出（读取），使用 `? extends T` | 方法参数用于读取 |
| **Consumer-Super** | 如果数据被消费（写入），使用 `? super T` | 方法参数用于写入 |

### 实际案例：Collections.copy

Java 标准库中的 `Collections.copy` 方法完美诠释了 PECS 原则：

```java
public static <T> void copy(List<? super T> dest, List<? extends T> src) {
    for (int i = 0; i < src.size(); i++) {
        dest.set(i, src.get(i));
    }
}
```

分析：
- `src`（源列表）是**生产者**，从中读取数据，所以用 `? extends T`
- `dest`（目标列表）是**消费者**，向其中写入数据，所以用 `? super T`

### 实战示例

```java
public class PECSExample {
    
    // Producer: 从列表中读取数据
    public static double sumOfList(List<? extends Number> list) {
        double sum = 0.0;
        for (Number n : list) {
            sum += n.doubleValue();
        }
        return sum;
    }
    
    // Consumer: 向列表中写入数据
    public static void addNumbers(List<? super Integer> list) {
        for (int i = 1; i <= 5; i++) {
            list.add(i);
        }
    }
    
    public static void main(String[] args) {
        // Producer 示例
        List<Integer> ints = Arrays.asList(1, 2, 3);
        List<Double> doubles = Arrays.asList(1.1, 2.2, 3.3);
        
        System.out.println(sumOfList(ints));    // 6.0
        System.out.println(sumOfList(doubles)); // 6.6
        
        // Consumer 示例
        List<Number> numbers = new ArrayList<>();
        addNumbers(numbers);
        System.out.println(numbers); // [1, 2, 3, 4, 5]
        
        List<Object> objects = new ArrayList<>();
        addNumbers(objects);
        System.out.println(objects); // [1, 2, 3, 4, 5]
    }
}
```

## 类型擦除（Type Erasure）

Java 泛型采用了**类型擦除**机制来实现向后兼容。在编译期，所有泛型信息都会被擦除，替换为它们的上界（通常是 `Object`）：

```java
// 编译前
List<String> strings = new ArrayList<>();
String s = strings.get(0);

// 编译后（类型擦除）
List strings = new ArrayList();
String s = (String) strings.get(0); // 自动插入类型转换
```

### 类型擦除的影响

1. **不能使用基本类型**：`List<int>` 是错误的，必须使用 `List<Integer>`
2. **运行时类型检查受限**：`instanceof List<String>` 是非法的
3. **不能创建泛型数组**：`new T[10]` 是非法的
4. **可以通过反射绕过泛型检查**：

```java
List<String> strings = new ArrayList<>();
// strings.add(42); // 编译错误

// 但可以通过反射绕过
Method m = strings.getClass().getMethod("add", Object.class);
m.invoke(strings, 42); // 运行时成功添加 Integer！
```

## 泛型与继承

泛型类型之间**不协变**（not covariant）：

```java
List<Object> objects = new ArrayList<String>(); // ❌ 编译错误
```

虽然 `String` 是 `Object` 的子类，但 `List<String>` 并不是 `List<Object>` 的子类。这是为了防止以下运行时错误：

```java
// 假设允许协变
List<String> strings = new ArrayList<>();
List<Object> objects = strings; // 假设可以
objects.add(42); // 向字符串列表中添加整数！
String s = strings.get(0); // ClassCastException！
```

## 总结

Java 泛型是类型安全的基石，掌握它需要理解以下核心概念：

| 概念 | 说明 | 示例 |
|------|------|------|
| 泛型类 | 类级别的类型参数 | `class Box<T>` |
| 泛型方法 | 方法级别的类型参数 | `<T> T method(T arg)` |
| `? extends T` | 上界通配符，用于读取 | `List<? extends Number>` |
| `? super T` | 下界通配符，用于写入 | `List<? super Integer>` |
| PECS | Producer-Extends, Consumer-Super | `copy(dest, src)` |
| 类型擦除 | 编译期擦除泛型信息 | `List<String>` → `List` |

> **最佳实践**：始终遵循 PECS 原则设计 API，使用通配符增加 API 的灵活性，但不要过度使用复杂的泛型嵌套，保持代码的可读性。

---

*本文首发于 Yggdrasil 博客*
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '2 days',
    NOW() - INTERVAL '2 days',
    NOW() - INTERVAL '2 days'
),
(
    1,
    'C++ RAII 与智能指针：现代 C++ 内存管理完全指南',
    'cpp-raii',
    '深入理解 C++ 的 RAII 原则、智能指针（unique_ptr、shared_ptr、weak_ptr）以及三/五/零法则，写出内存安全的现代 C++ 代码。',
    $doc$
# C++ RAII 与智能指针：现代 C++ 内存管理完全指南

C++ 是一门赋予程序员极大自由度的语言，这种自由既带来了高性能，也带来了内存管理的挑战。手动管理内存容易引发内存泄漏、悬垂指针、双重释放等问题。为了解决这些问题，C++ 社区发展出了 **RAII**（Resource Acquisition Is Initialization，资源获取即初始化）原则，以及一套智能指针工具。掌握这些现代 C++ 特性，是编写健壮 C++ 代码的关键。

本文将系统介绍 RAII 原则、智能指针的使用、三/五/零法则，以及实际开发中的最佳实践。

## 什么是 RAII？

RAII 是 C++ 的核心编程范式，由 Bjarne Stroustrup 提出。其核心思想是：**将资源的生命周期与对象的生命周期绑定，在对象构造时获取资源，在对象析构时释放资源**。

由于 C++ 的栈对象在离开作用域时会自动调用析构函数，这一机制天然地保证了资源的正确释放，即使在发生异常的情况下也是如此。

### 基础示例：文件句柄

```cpp
#include <cstdio>
#include <stdexcept>

class FileHandle {
    FILE* file;
    
public:
    // 构造函数：获取资源
    explicit FileHandle(const char* filename, const char* mode = "r") 
        : file(std::fopen(filename, mode)) {
        if (!file) {
            throw std::runtime_error("无法打开文件");
        }
    }
    
    // 析构函数：释放资源
    ~FileHandle() {
        if (file) {
            std::fclose(file);
        }
    }
    
    // 禁用拷贝（资源不可复制）
    FileHandle(const FileHandle&) = delete;
    FileHandle& operator=(const FileHandle&) = delete;
    
    // 允许移动（C++11）
    FileHandle(FileHandle&& other) noexcept : file(other.file) {
        other.file = nullptr;
    }
    
    FileHandle& operator=(FileHandle&& other) noexcept {
        if (this != &other) {
            if (file) std::fclose(file);
            file = other.file;
            other.file = nullptr;
        }
        return *this;
    }
    
    FILE* get() const { return file; }
};

// 使用
void processFile(const char* filename) {
    FileHandle fh(filename); // 打开文件
    // 处理文件...
    // 即使发生异常，fh 的析构函数也会确保文件被关闭
}
```

## 三/五/零法则

### 三法则（Rule of Three）

在 C++98/03 时代，如果一个类需要自定义以下三个函数中的任意一个，通常需要定义全部三个：

1. **析构函数**（Destructor）
2. **拷贝构造函数**（Copy Constructor）
3. **拷贝赋值运算符**（Copy Assignment Operator）

这是因为这三个函数都与资源管理密切相关。如果你需要自定义其中一个，说明你的类管理着某种资源，因此三个都需要自定义。

### 五法则（Rule of Five）

C++11 引入了移动语义，五法则在三法则基础上增加了：

4. **移动构造函数**（Move Constructor）
5. **移动赋值运算符**（Move Assignment Operator）

```cpp
class Resource {
    int* data;
    size_t size;
    
public:
    // 构造函数
    explicit Resource(size_t n) : data(new int[n]), size(n) {}
    
    // 1. 析构函数
    ~Resource() {
        delete[] data;
    }
    
    // 2. 拷贝构造函数
    Resource(const Resource& other) : data(new int[other.size]), size(other.size) {
        std::copy(other.data, other.data + size, data);
    }
    
    // 3. 拷贝赋值运算符
    Resource& operator=(const Resource& other) {
        if (this != &other) {
            Resource temp(other); // 拷贝并交换惯用法
            std::swap(data, temp.data);
            std::swap(size, temp.size);
        }
        return *this;
    }
    
    // 4. 移动构造函数
    Resource(Resource&& other) noexcept : data(other.data), size(other.size) {
        other.data = nullptr;
        other.size = 0;
    }
    
    // 5. 移动赋值运算符
    Resource& operator=(Resource&& other) noexcept {
        if (this != &other) {
            delete[] data;
            data = other.data;
            size = other.size;
            other.data = nullptr;
            other.size = 0;
        }
        return *this;
    }
};
```

### 零法则（Rule of Zero）

现代 C++ 的最佳实践是**零法则**：

> **如果一个类不需要自定义析构函数、拷贝/移动构造函数或赋值运算符，那就不要自定义。**

通过使用智能指针和标准库容器，你可以让编译器自动生成这些函数：

```cpp
// ✅ 遵循零法则
class ModernResource {
    std::unique_ptr<int[]> data;
    size_t size;
    
public:
    explicit ModernResource(size_t n) : data(std::make_unique<int[]>(n)), size(n) {}
    
    // 编译器自动生成的析构函数、拷贝/移动函数都能正确工作
    // 因为 unique_ptr 已经正确管理了内存
};
```

## 智能指针

C++11 引入了三种智能指针，分别适用于不同的所有权模型：

### std::unique_ptr：独占所有权

`unique_ptr` 表示对对象的**独占所有权**，同一时间只能有一个 `unique_ptr` 指向给定对象。当 `unique_ptr` 被销毁时，它所指向的对象也会被自动删除。

```cpp
#include <memory>
#include <iostream>

class Widget {
public:
    Widget() { std::cout << "Widget 构造\\n"; }
    ~Widget() { std::cout << "Widget 析构\\n"; }
    void doSomething() { std::cout << "Widget 工作中\\n"; }
};

void uniquePtrDemo() {
    // 创建 unique_ptr
    std::unique_ptr<Widget> ptr1 = std::make_unique<Widget>();
    ptr1->doSomething();
    
    // 转移所有权
    std::unique_ptr<Widget> ptr2 = std::move(ptr1);
    // ptr1 现在为空
    // ptr2->doSomething(); // ✅
    
    // 自动释放
} // Widget 在此处被销毁

// 工厂函数返回 unique_ptr
std::unique_ptr<Widget> createWidget() {
    return std::make_unique<Widget>();
}
```

**最佳实践**：
- 默认使用 `std::make_unique` 创建（C++14）
- 用于表示独占所有权
- 作为函数参数传递时，使用 `std::move` 转移所有权

### std::shared_ptr：共享所有权

`shared_ptr` 通过**引用计数**实现共享所有权。多个 `shared_ptr` 可以同时指向同一个对象，当最后一个 `shared_ptr` 被销毁时，对象才被删除。

```cpp
#include <memory>
#include <iostream>

void sharedPtrDemo() {
    // 创建 shared_ptr
    std::shared_ptr<Widget> ptr1 = std::make_shared<Widget>();
    {
        std::shared_ptr<Widget> ptr2 = ptr1; // 引用计数 +1
        std::cout << "引用计数: " << ptr1.use_count() << "\\n"; // 2
    } // ptr2 销毁，引用计数 -1
    
    std::cout << "引用计数: " << ptr1.use_count() << "\\n"; // 1
} // Widget 在此处被销毁
```

### std::weak_ptr：弱引用

`weak_ptr` 是一种**不控制对象生命周期**的智能指针。它指向一个由 `shared_ptr` 管理的对象，但不会增加引用计数。它主要用于解决 **循环引用** 问题：

```cpp
#include <memory>
#include <iostream>

class B; // 前向声明

class A {
public:
    std::shared_ptr<B> b_ptr;
    ~A() { std::cout << "A 析构\\n"; }
};

class B {
public:
    // 使用 weak_ptr 避免循环引用
    std::weak_ptr<A> a_ptr;
    ~B() { std::cout << "B 析构\\n"; }
};

void weakPtrDemo() {
    {
        std::shared_ptr<A> a = std::make_shared<A>();
        std::shared_ptr<B> b = std::make_shared<B>();
        
        a->b_ptr = b;
        b->a_ptr = a; // weak_ptr，不增加引用计数
        
        // 检查对象是否还存在
        if (auto shared = b->a_ptr.lock()) {
            std::cout << "A 仍然存在\\n";
        }
    } // A 和 B 都被正确析构
}
```

## 智能指针对比

| 特性 | `unique_ptr` | `shared_ptr` | `weak_ptr` |
|------|-------------|-------------|-----------|
| 所有权 | 独占 | 共享 | 无（弱引用） |
| 引用计数 | 无 | 有 | 无 |
| 可拷贝 | ❌ | ✅ | ✅ |
| 可移动 | ✅ | ✅ | ✅ |
| 内存开销 | 最小 | 引用计数 + 控制块 | 最小 |
| 适用场景 | 独占资源 | 共享资源 | 打破循环引用 |

## RAII 在标准库中的应用

C++ 标准库大量使用了 RAII 原则：

```cpp
// 容器
std::vector<int> vec = {1, 2, 3}; // 自动管理内存

// 锁
std::mutex mtx;
{
    std::lock_guard<std::mutex> lock(mtx); // 自动加锁
    // 临界区...
} // 自动解锁

// 文件流
{
    std::ofstream file("data.txt");
    file << "Hello, RAII!";
} // 自动关闭文件

// 线程
{
    std::thread t([]() {
        std::cout << "后台任务\\n";
    });
    t.join(); // 或者使用 RAII 包装器
}
```

## 总结

现代 C++ 的内存管理已经不再是噩梦。通过遵循以下原则，你可以写出既高效又安全的代码：

1. **优先使用智能指针**：`unique_ptr` > `shared_ptr` > 原始指针
2. **遵循零法则**：使用标准库工具管理资源
3. **使用 `std::make_unique` 和 `std::make_shared`**：异常安全且高效
4. **理解所有权语义**：明确谁拥有资源，谁只是借用
5. **注意循环引用**：使用 `weak_ptr` 打破循环

> **C++ 之父的名言**："C++ 中，资源管理即对象管理。" —— Bjarne Stroustrup

---

*本文首发于 Yggdrasil 博客*
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '1 day',
    NOW() - INTERVAL '1 day',
    NOW() - INTERVAL '1 day'
),
(
    1,
    'Haskell 纯函数、惰性求值与 Monad：函数式编程的精髓',
    'haskell-pure-functions',
    '深入理解 Haskell 的纯函数、惰性求值、高阶函数和 Monad，探索函数式编程的核心思想。',
    $doc$
# Haskell 纯函数、惰性求值与 Monad：函数式编程的精髓

Haskell 是一门**纯函数式编程语言**，以严格的数学基础和优雅的设计著称。与命令式语言不同，Haskell 强调函数的数学本质：**函数只是从输入到输出的映射**，没有副作用、没有状态变化、没有隐式的执行顺序。这种纯粹性带来了前所未有的代码可预测性和可组合性。

本文将深入探讨 Haskell 的三大核心特性：纯函数、惰性求值和 Monad，帮助你理解函数式编程的精髓。

## 纯函数（Pure Functions）

纯函数是函数式编程的基石。一个函数是纯函数，当且仅当满足两个条件：

1. **引用透明**（Referential Transparency）：对于相同的输入，永远返回相同的输出
2. **无副作用**（No Side Effects）：不修改外部状态，不执行 I/O 操作

### 纯函数示例

```haskell
-- 纯函数：给定输入，总有确定的输出
factorial :: Integer -> Integer
factorial 0 = 1
factorial n = n * factorial (n - 1)

-- 另一个纯函数
square :: Num a => a -> a
square x = x * x

-- 纯函数可以安全地被替换为其结果（引用透明）
-- square 5 总是可以被替换为 25
```

### 纯函数的优势

| 特性 | 说明 |
|------|------|
| 可测试性 | 无需模拟外部状态，输入确定即可测试 |
| 可缓存性 | 结果可以被记忆化（memoization） |
| 可并行性 | 无共享状态，天然适合并行计算 |
| 可组合性 | 函数可以像乐高积木一样组合 |

## 高阶函数（Higher-Order Functions）

Haskell 中的函数是一等公民，可以作为参数传递，也可以作为返回值：

```haskell
-- map：对列表每个元素应用函数
map :: (a -> b) -> [a] -> [b]
doubleAll = map (* 2)
-- doubleAll [1, 2, 3] => [2, 4, 6]

-- filter：根据条件过滤列表
filter :: (a -> Bool) -> [a] -> [a]
evens = filter even
-- evens [1..10] => [2, 4, 6, 8, 10]

-- foldl / foldr：列表归约
sumList = foldl (+) 0
-- sumList [1, 2, 3, 4] => 10

-- 函数组合
(.) :: (b -> c) -> (a -> b) -> a -> c
f . g = \\x -> f (g x)

-- 使用函数组合
process = sum . map square . filter even
-- process [1..10] = sum (map square (filter even [1..10]))
```

### 自定义高阶函数

```haskell
-- 将函数应用两次
applyTwice :: (a -> a) -> a -> a
applyTwice f x = f (f x)

-- 使用
applyTwice (+ 3) 10  -- 16
applyTwice reverse [1, 2, 3]  -- [1, 2, 3]

-- 柯里化（Currying）
add :: Int -> Int -> Int
add x y = x + y

-- add 5 是一个接收 Int 返回 Int 的函数
addFive = add 5
addFive 3  -- 8
```

## 惰性求值（Lazy Evaluation）

Haskell 默认采用**惰性求值**（Lazy Evaluation），也称为**按需调用**（Call by Need）。这意味着表达式在真正需要其值时才会被计算。

### 无限列表

惰性求值最惊人的特性之一，就是可以定义**无限数据结构**：

```haskell
-- 无限自然数列表
nats :: [Integer]
nats = [0..]

-- 无限偶数列表
evens :: [Integer]
evens = [0, 2..]

-- 无限斐波那契数列
fibs :: [Integer]
fibs = 0 : 1 : zipWith (+) fibs (tail fibs)

-- 使用 take 只取有限部分
main = do
    print $ take 10 nats       -- [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    print $ take 10 evens      -- [0, 2, 4, 6, 8, 10, 12, 14, 16, 18]
    print $ take 15 fibs       -- [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377]
```

### 列表推导式

Haskell 的列表推导式（List Comprehension）是处理集合的强大工具：

```haskell
-- 基本列表推导式
squares = [x^2 | x <- [1..10]]
-- [1, 4, 9, 16, 25, 36, 49, 64, 81, 100]

-- 带过滤条件
evenSquares = [x^2 | x <- [1..20], even x]
-- [4, 16, 36, 64, 100, 144, 196, 256, 324, 400]

-- 多生成器
pairs = [(x, y) | x <- [1..3], y <- ['a', 'b']]
-- [(1,'a'), (1,'b'), (2,'a'), (2,'b'), (3,'a'), (3,'b')]

-- 使用 let 绑定
result = [let y = x * 2 in (x, y) | x <- [1..5]]
-- [(1,2), (2,4), (3,6), (4,8), (5,10)]
```

### 惰性求值的实际应用

```haskell
-- 只计算需要的部分
firstEvenSquareOver100 = head [x^2 | x <- [1..], even x, x^2 > 100]
-- 结果为 144（12^2），不需要计算 [1..] 的所有元素

-- 短路求值
and' :: [Bool] -> Bool
and' = foldr (&&) True
-- and' (False : undefined) => False
-- 不需要计算 undefined，因为第一个 False 已经决定了结果
```

## 类型系统与类型类

Haskell 拥有强大的静态类型系统和类型推断能力：

```haskell
-- 代数数据类型（Algebraic Data Types）
data Shape = Circle Float
           | Rectangle Float Float
           | Triangle Float Float Float
           deriving (Show, Eq)

area :: Shape -> Float
area (Circle r) = pi * r * r
area (Rectangle w h) = w * h
area (Triangle a b c) = 
    let s = (a + b + c) / 2
    in sqrt (s * (s - a) * (s - b) * (s - c))

-- 类型类（Type Classes）
class Describable a where
    describe :: a -> String

instance Describable Shape where
    describe (Circle r) = "半径为 " ++ show r ++ " 的圆"
    describe (Rectangle w h) = show w ++ " x " ++ show h ++ " 的矩形"
```

## Monad：处理副作用的优雅方式

纯函数不能执行 I/O 操作，但程序总要与外界交互。Haskell 使用 **Monad** 来在纯函数式框架内处理副作用。

### Maybe Monad：处理可能失败的计算

```haskell
-- 安全除法
safeDiv :: Int -> Int -> Maybe Int
safeDiv _ 0 = Nothing
safeDiv a b = Just (a `div` b)

-- 使用 do 语法串联 Maybe 计算
calculate :: Int -> Int -> Int -> Maybe Int
calculate x y z = do
    a <- safeDiv x y    -- 如果失败，整个计算返回 Nothing
    b <- safeDiv a z
    return (b + 1)

-- 示例
calculate 10 2 3  -- Just 2
calculate 10 0 3  -- Nothing
calculate 10 2 0  -- Nothing
```

### IO Monad：处理输入输出

```haskell
-- IO 是一个 Monad，将副作用操作封装起来
greet :: IO ()
greet = do
    putStrLn "请输入你的名字:"
    name <- getLine
    putStrLn $ "你好, " ++ name ++ "!"

-- 使用 >>=（bind）操作符
main :: IO ()
main = putStrLn "Hello" >>= \_ -> putStrLn "World"
```

### 常用 Monad

| Monad | 用途 | 示例 |
|-------|------|------|
| `Maybe` | 可能失败的计算 | `safeDiv`、`lookup` |
| `Either` | 带错误信息的计算 | `parseNumber` |
| `IO` | 输入输出操作 | `readFile`、`putStrLn` |
| `List` | 非确定性计算 | 多个结果 |
| `State` | 状态传递 | 计数器、随机数生成 |
| `Reader` | 环境读取 | 配置读取 |
| `Writer` | 日志记录 | 追踪计算过程 |

## 总结

Haskell 的函数式编程范式带来了全新的编程思维方式：

| 概念 | 核心思想 | 优势 |
|------|---------|------|
| 纯函数 | 无副作用、引用透明 | 可预测、可测试、可并行 |
| 惰性求值 | 按需计算 | 无限数据结构、短路求值 |
| 高阶函数 | 函数作为一等公民 | 强大的组合能力 |
| 类型系统 | 强类型、类型推断 | 编译期捕获错误 |
| Monad | 在纯函数框架内处理副作用 | 优雅的副作用管理 |

> "学习 Haskell 不是为了在工作中使用它，而是为了以全新的方式思考编程。" —— 匿名

Haskell 可能不是最适合所有场景的语言，但它所倡导的函数式编程思想——不可变性、纯函数、组合——已经深刻影响了现代编程语言的设计。从 JavaScript 的 `map`/`filter`/`reduce`，到 Java 的 Stream API，再到 Rust 的迭代器，函数式编程的思想无处不在。

---

*本文首发于 Yggdrasil 博客*
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '12 hours',
    NOW() - INTERVAL '12 hours',
    NOW() - INTERVAL '12 hours'
),
(
    1,
    'Ruby 元编程：打开动态语言的黑箱',
    'ruby-metaprogramming',
    'Ruby 被誉为"程序员的最好朋友"，其强大的元编程能力让代码更加灵活和富有表现力。本文深入探讨 define_method、method_missing、类_eval、模块混入等核心元编程技术。',
    $doc$
# Ruby 元编程：打开动态语言的黑箱

Ruby 是一门充满魅力的动态语言，Matz（松本行弘）在设计 Ruby 时的理念是：**让编程变得快乐**。这种快乐很大程度上来自于 Ruby 强大的**元编程**（Metaprogramming）能力——即编写能够编写代码的代码。

元编程并非 Ruby 独有，但 Ruby 的元编程能力在众多语言中出类拔萃。Rails 框架的成功很大程度上归功于其巧妙运用元编程实现的"约定优于配置"哲学。

本文将深入探讨 Ruby 元编程的核心技术，帮助你理解这门动态语言的本质。

## 什么是元编程？

元编程是指在运行时创建、修改或分析代码的技术。在 Ruby 中，几乎所有事物都是对象，包括类本身。这意味着你可以像操作普通对象一样操作类——添加方法、修改方法、甚至在运行时创建全新的类。

```ruby
# 类也是对象！
puts String.class      # => Class
puts Class.class       # => Class
puts Object.class      # => Class
```

## 动态方法定义

### define_method

`define_method` 允许在运行时动态定义方法：

```ruby
class Robot
  ACTIONS = [:walk, :run, :jump, :fly]

  ACTIONS.each do |action|
    define_method(action) do |*args|
      speed = args.first || "normal"
      "🤖 Robot is #{action}ing at #{speed} speed!"
    end
  end
end

robot = Robot.new
puts robot.walk         # => 🤖 Robot is walking at normal speed!
puts robot.run("fast")  # => 🤖 Robot is running at fast speed!
puts robot.fly("super") # => 🤖 Robot is flying at super speed!
```

这种技术在框架开发中极为常见。例如，ActiveRecord 的 `find_by_*` 方法就是动态生成的。

### method_missing：拦截不存在的方法

`method_missing` 是 Ruby 元编程中最强大也最危险的工具。当调用一个不存在的方法时，Ruby 会将调用转发给 `method_missing`：

```ruby
class DynamicFinder
  def method_missing(name, *args, &block)
    if name.to_s.start_with?("find_by_")
      attribute = name.to_s.sub("find_by_", "")
      "Looking for record where #{attribute} = #{args.first}"
    else
      super # 如果不是我们处理的格式，交给父类处理
    end
  end

  def respond_to_missing?(name, include_private = false)
    name.to_s.start_with?("find_by_") || super
  end
end

finder = DynamicFinder.new
puts finder.find_by_name("Alice")   # => Looking for record where name = Alice
puts finder.find_by_email("a@b.c") # => Looking for record where email = a@b.c
```

### const_missing：动态加载常量

```ruby
module AutoLoader
  def self.const_missing(name)
    file = name.to_s.downcase
    require_relative "./#{file}"
    const_get(name)
  rescue LoadError
    super
  end
end
```

## 打开类（Open Classes）

Ruby 允许随时重新打开已存在的类并添加或修改方法，这被称为**猴子补丁**（Monkey Patching）：

```ruby
class String
  def shout
    upcase + "!!!"
  end

  def reverse_words
    split.reverse.join(" ")
  end

  def to_slug
    downcase.strip.gsub(/\s+/, '-').gsub(/[^\w-]/, '')
  end
end

puts "hello world".shout           # => HELLO WORLD!!!
puts "hello world".reverse_words   # => world hello
puts "Hello World!".to_slug        # => hello-world
```

### 使用 Refinement 安全地扩展

为了避免猴子补丁污染全局命名空间，Ruby 2.0 引入了 **Refinement**：

```ruby
module StringExtensions
  refine String do
    def shout
      upcase + "!!!"
    end
  end
end

class MyApp
  using StringExtensions

  def self.greet(name)
    name.shout
  end
end

puts MyApp.greet("hello")  # => HELLO!!!
puts "hello".shout rescue puts "No method error"  # => No method error
```

## 类_eval 和 instance_eval

Ruby 提供了多种在特定上下文中执行代码的方式：

### class_eval（或 module_eval）

在类的上下文中执行代码，可以访问类的私有方法：

```ruby
class Person
  def initialize(name)
    @name = name
  end
end

# 为 Person 类动态添加方法
Person.class_eval do
  attr_reader :name

  def greet
    "Hello, I'm #{@name}!"
  end

  def self.species
    "Homo sapiens"
  end
end

person = Person.new("Alice")
puts person.name        # => Alice
puts person.greet       # => Hello, I'm Alice!
puts Person.species     # => Homo sapiens
```

### instance_eval

在对象的上下文中执行代码：

```ruby
class Dog
  def initialize(name)
    @name = name
  end
end

dog = Dog.new("Buddy")

dog.instance_eval do
  def bark
    "#{@name} says: Woof!"
  end
end

puts dog.bark  # => Buddy says: Woof!
# 注意：bark 方法只存在于这个实例上，其他 Dog 实例没有
```

### instance_exec（带参数）

```ruby
class Calculator
  def initialize(value)
    @value = value
  end
end

calc = Calculator.new(10)
result = calc.instance_exec(5) do |n|
  @value + n
end
puts result  # => 15
```

## 模块与混入（Mixins）

Ruby 不支持多重继承，但通过模块混入实现了更灵活的功能复用：

```ruby
module Loggable
  def log(message)
    puts "[#{Time.now}] #{self.class}: #{message}"
  end

  def self.included(base)
    puts "#{base} included #{self}"
  end
end

module Validatable
  def validate!
    raise "Invalid!" unless valid?
  end
end

class User
  include Loggable   # 实例方法
  extend Validatable # 类方法

  def valid?
    true
  end
end

user = User.new
user.log("Created")  # => [2024-...] User: Created
```

### prepend：方法前置

Ruby 2.0 引入了 `prepend`，可以将模块的方法插入到类的方法链前面：

```ruby
module Logging
  def save
    puts "Before save..."
    super
    puts "After save..."
  end
end

class Article
  prepend Logging

  def save
    puts "Saving article..."
  end
end

article = Article.new
article.save
# => Before save...
# => Saving article...
# => After save...
```

## 元编程在 Rails 中的应用

Rails 是 Ruby 元编程能力的最佳展示：

```ruby
# ActiveRecord 动态属性访问
user = User.find(1)
user.name           # 动态生成 getter
user.name = "Alice" # 动态生成 setter
user.save           # 动态生成 SQL

# 关联宏
class User < ApplicationRecord
  has_many :posts
  has_many :comments
  belongs_to :company
end

# 这背后使用元编程动态创建了 posts、comments、company 等方法
```

## 元编程最佳实践

| 技术 | 适用场景 | 注意事项 |
|------|---------|---------|
| `define_method` | 批量生成相似方法 | 比 `class_eval` 更清洁 |
| `method_missing` | 实现动态 API | 始终实现 `respond_to_missing?` |
| `class_eval` | 动态修改类 | 会改变类的全局行为 |
| `instance_eval` | DSL 设计 | 注意 `self` 的变化 |
| Refinement | 安全扩展内置类 | 只在 `using` 的作用域内生效 |

## 总结

Ruby 的元编程能力让这门语言充满了表达力和灵活性。通过动态方法定义、方法拦截、类修改和模块混入，你可以写出极其简洁和富有表现力的代码。

但元编程也是一把双刃剑：

- **优点**：代码更 DRY、更表达力、框架更强大
- **缺点**：调试困难、IDE 支持有限、可读性降低

> "Ruby 让编程变得有趣，但元编程让 Ruby 变得强大。" —— 改编自 Matz

在使用元编程时，请始终牢记：**清晰的代码胜过聪明的代码**。元编程应该用来消除重复、提高抽象层次，而不是用来炫耀技巧。

---

*本文首发于 Yggdrasil 博客*
$doc$,
    NULL,
    'draft',
    NULL,
    NOW(),
    NOW()
),
(
    1,
    'Zig 显式内存管理与编译期编程：系统级语言的新选择',
    'zig-memory-management',
    'Zig 是一门注重可读性、健壮性和最优性的系统级编程语言。没有隐式内存分配、没有隐藏的控制流，让每一行代码都清晰可控。',
    $doc$
# Zig 显式内存管理与编译期编程：系统级语言的新选择

Zig 是一门相对年轻的系统级编程语言，由 Andrew Kelley 于 2016 年开始开发。它的设计哲学可以概括为：**显式优于隐式**、**可读性优先于奇技淫巧**、**健壮性优先于方便性**。

与 C 语言相比，Zig 提供了更好的安全性、更强大的元编程能力和更友好的构建系统。与 Rust 相比，Zig 更加简单直接，没有复杂的所有权系统，而是通过显式的内存分配和错误处理来保证安全。

本文将深入探讨 Zig 的核心特性，包括显式内存管理、错误处理、编译期编程和 C 互操作性。

## 显式内存分配

Zig 最显著的特点之一是**没有隐式内存分配**。在 Zig 中，所有可能分配内存的操作都必须显式地接收一个分配器参数。

### 基础内存分配

```zig
const std = @import("std");

pub fn main() !void {
    // 创建通用分配器
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    // 动态分配内存
    const memory = try allocator.alloc(u8, 100);
    defer allocator.free(memory);  // 确保内存被释放

    // 使用内存...
    @memcpy(memory, "Hello, Zig!");
    std.debug.print("{s}\n", .{memory});
}
```

### 不同的分配器策略

Zig 标准库提供了多种分配器，适用于不同场景：

| 分配器 | 用途 | 特点 |
|--------|------|------|
| `GeneralPurposeAllocator` | 通用场景 | 安全、支持内存泄漏检测 |
| `PageAllocator` | 大块内存 | 直接向 OS 申请 |
| `FixedBufferAllocator` | 嵌入式/无堆环境 | 使用预分配缓冲区 |
| `ArenaAllocator` | 批量分配/释放 | 一次性释放所有内存 |
| `c_allocator` | C 互操作 | 使用 malloc/free |

```zig
// 使用 Arena 分配器
var arena = std.heap.ArenaAllocator.init(std.heap.page_allocator);
defer arena.deinit();
const allocator = arena.allocator();

// 分配多个对象
const str1 = try allocator.dupe(u8, "Hello");
const str2 = try allocator.dupe(u8, "World");
// 所有内存会在 arena.deinit() 时一次性释放
```

### 错误处理与内存安全

Zig 的 `try` 关键字确保了错误被传播，而 `defer` 确保了资源被释放，即使在错误路径上：

```zig
fn processData(allocator: std.mem.Allocator, input: []const u8) !void {
    const buffer = try allocator.alloc(u8, input.len * 2);
    defer allocator.free(buffer);  // 无论成功与否都会执行

    if (input.len == 0) {
        return error.EmptyInput;  // buffer 仍会被释放
    }

    // 处理数据...
    @memcpy(buffer[0..input.len], input);
}
```

## 错误处理

Zig 的错误处理机制结合了错误联合类型（Error Union Types）和 `try/catch` 风格：

### 错误联合类型

```zig
const FileOpenError = error{
    AccessDenied,
    OutOfMemory,
    FileNotFound,
    NotAFile,
};

fn openFile(path: []const u8) FileOpenError!std.fs.File {
    return std.fs.cwd().openFile(path, .{});
}
```

`FileOpenError!std.fs.File` 表示这个函数要么返回一个错误（`FileOpenError`），要么返回一个文件（`std.fs.File`）。

### try 与 catch

```zig
pub fn main() !void {
    // 使用 try：如果出错，立即返回错误
    const file = try openFile("data.txt");
    defer file.close();

    // 使用 catch：处理特定错误
    const alt_file = openFile("backup.txt") catch |err| {
        std.debug.print("打开备份失败: {}\n", .{err});
        return;
    };
    defer alt_file.close();

    // 使用 catch 提供默认值
    const content = readFile("config.txt") catch "default_config";
}
```

### errdefer

`errdefer` 只在函数返回错误时执行，非常适合错误路径上的清理：

```zig
fn createUser(allocator: std.mem.Allocator, name: []const u8) !User {
    const user_name = try allocator.dupe(u8, name);
    errdefer allocator.free(user_name);  // 只有出错时才释放

    const user_email = try allocator.dupe(u8, "default@example.com");
    errdefer allocator.free(user_email);

    return User{
        .name = user_name,
        .email = user_email,
    };
}
```

## 编译期编程（Comptime）

Zig 的 `comptime` 关键字允许在编译期执行代码，这是 Zig 最强大的特性之一：

### 编译期计算

```zig
fn fibonacci(comptime n: u32) u32 {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

const fib_10 = comptime fibonacci(10);  // 编译期计算！
```

### 类型作为参数

```zig
fn Vector(comptime T: type, comptime n: comptime_int) type {
    return struct {
        data: [n]T,

        pub fn add(self: @This(), other: @This()) @This() {
            var result: @This() = undefined;
            for (&result.data, self.data, other.data) |*r, a, b| {
                r.* = a + b;
            }
            return result;
        }
    };
}

const Vec3f = Vector(f32, 3);
var v1 = Vec3f{ .data = .{ 1, 2, 3 } };
var v2 = Vec3f{ .data = .{ 4, 5, 6 } };
const v3 = v1.add(v2);
```

### 编译期反射

```zig
fn printStructInfo(comptime T: type) void {
    const info = @typeInfo(T);
    std.debug.print("Type: {s}\n", .{@typeName(T)});

    if (info == .Struct) {
        std.debug.print("Fields:\n");
        inline for (info.Struct.fields) |field| {
            std.debug.print("  {s}: {}\n", .{ field.name, field.type });
        }
    }
}
```

## C 互操作性

Zig 与 C 的互操作性非常出色，可以直接导入 C 头文件并调用 C 函数：

```zig
const c = @cImport({
    @cInclude("stdio.h");
    @cInclude("stdlib.h");
});

pub fn main() void {
    c.printf("Hello from C!\n");

    const ptr = c.malloc(100);
    defer c.free(ptr);

    // Zig 可以直接处理 C 指针
    const zig_slice = @as([*]u8, @ptrCast(ptr))[0..100];
}
```

## Zig 的哲学

| 原则 | 说明 | 示例 |
|------|------|------|
| **显式优于隐式** | 没有隐式分配、没有隐式转换 | 所有分配器必须显式传递 |
| **无隐藏控制流** | 没有运算符重载、没有隐式析构 | 资源释放用 `defer` |
| **无隐藏分配** | 内存分配一目了然 | `ArrayList` 需要传入分配器 |
| **可读性优先** | 代码应该像散文一样易读 | 简洁的语法，无冗余符号 |
| **与 C 互操作** | 无缝集成现有 C 代码库 | `@cImport` 直接导入头文件 |

## 总结

Zig 是一门为系统编程而生的现代语言，它吸收了 C 的简洁和 Rust 的安全理念，同时保持了自己的独特风格：

1. **显式内存管理**：没有垃圾回收，没有隐式分配
2. **强大的错误处理**：错误联合类型 + try/catch
3. **编译期编程**：`comptime` 提供元编程能力
4. **出色的 C 互操作**：无缝集成现有生态系统
5. **简单直接**：没有复杂的所有权系统，只有清晰的规则

> **Zig 的设计理念**："专注于调试你的应用程序而不是调试你的编程语言知识。"

对于需要高性能、低级别控制、但又不想处理 C 语言种种陷阱的项目，Zig 是一个极具吸引力的选择。

---

*本文首发于 Yggdrasil 博客*
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '6 hours',
    NOW() - INTERVAL '6 hours',
    NOW() - INTERVAL '6 hours'
),
(
    1,
    'Markdown 语法完全测试：渲染引擎兼容性验证',
    'markdown-syntax-test',
    '一篇专门用于测试博客系统对 Markdown 各种语法元素支持情况的文章，包含文本格式、列表、代码块、表格、数学公式、脚注等完整测试。',
    $doc$
# Markdown 语法完全测试：渲染引擎兼容性验证

这是一篇专门用于测试 Markdown 渲染效果的文章。无论你使用的是 CommonMark、GitHub Flavored Markdown (GFM)、还是其他扩展方言，本文都涵盖了最全面的语法测试用例。

Markdown 的设计目标是**易读易写**，其语法灵感来源于纯文本电子邮件的格式。本文不仅展示各种语法元素，还说明了它们的使用场景和最佳实践。

## 文本格式

Markdown 支持多种内联文本格式：

这是**粗体**（使用 `**` 或 `__` 包裹），这是*斜体*（使用 `*` 或 `_` 包裹），这是***粗斜体***（同时使用两者），这是~~删除线~~（使用 `~~` 包裹），这是`行内代码`（使用反引号包裹）。

### 强调嵌套

你可以嵌套不同的格式：这是**粗体中的_斜体_**，这是*斜体中的**粗体***。

### 转义字符

如果需要显示 Markdown 语法字符本身，可以使用反斜杠转义：\*这不是斜体\*，\`这不是代码\`。

## 标题层级

Markdown 支持六级标题，使用 `#` 的数量表示层级：

### 三级标题

三级标题常用于文章的主要小节。

#### 四级标题

四级标题用于更细分的子节。

##### 五级标题

五级标题用于细节描述。

###### 六级标题

六级标题是最深层级，一般用于列表项内的标题。

## 列表

### 无序列表

无序列表使用 `-`、`+` 或 `*` 作为标记符：

- 第一项：这是列表的第一项
- 第二项：包含子列表
  - 嵌套项 1：使用两个空格缩进
  - 嵌套项 2：可以继续嵌套
    - 更深嵌套：三层缩进
    - 混合使用不同标记符也可以
- 第三项：回到第一层

### 有序列表

有序列表使用数字加句点：

1. 第一步：准备工作
2. 第二步：执行操作
   1. 子步骤 A：检查环境
   2. 子步骤 B：执行命令
3. 第三步：验证结果

### 任务列表

GitHub Flavored Markdown 支持任务列表：

- [x] 已完成任务：设置开发环境
- [x] 已完成任务：编写核心代码
- [ ] 未完成任务：编写单元测试
- [ ] 未完成任务：部署到生产环境
- [x] 已完成任务：编写文档

## 引用块

引用块用于引用他人的话或强调重要内容：

> 这是一段普通的引用块。引用块可以包含多行内容，
> 每一行都以 `>` 开头。
>
> > 这是嵌套引用块。在引用中再引用，形成层级结构。
> > 这在回复邮件或讨论中非常常见。
>
> 回到第一层引用，继续你的论述。

### 引用块中的其他元素

> **注意**：引用块中可以包含其他 Markdown 元素。
>
> - 比如列表项
> - 比如 `行内代码`
> - 比如 [链接](https://example.com)
>
> ```python
> # 甚至可以包含代码块
> print("在引用块中执行代码")
> ```

## 代码块

代码块是技术文章的核心。Markdown 支持多种方式展示代码。

### Python

```python
def hello_world(name: str = "World") -> str:
    """返回问候语。"""
    return f"Hello, {name}!"

class Greeter:
    def __init__(self, greeting: str = "Hello"):
        self.greeting = greeting
    
    def greet(self, name: str) -> str:
        return f"{self.greeting}, {name}!"

if __name__ == "__main__":
    greeter = Greeter("Hi")
    print(greeter.greet("Alice"))
```

### Rust

```rust
fn main() {
    let name = "world";
    println!("Hello, {}!", name);
    
    let numbers = vec![1, 2, 3, 4, 5];
    let sum: i32 = numbers.iter().sum();
    println!("Sum: {}", sum);
}
```

### JavaScript

```javascript
async function fetchUserData(userId) {
    try {
        const response = await fetch(`/api/users/${userId}`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    } catch (error) {
        console.error("Failed to fetch user:", error);
        throw error;
    }
}
```

### Go

```go
package main

import (
    "fmt"
    "time"
)

func worker(id int, jobs <-chan int, results chan<- int) {
    for j := range jobs {
        fmt.Printf("Worker %d processing job %d\n", id, j)
        time.Sleep(time.Second)
        results <- j * 2
    }
}

func main() {
    jobs := make(chan int, 100)
    results := make(chan int, 100)
    
    for w := 1; w <= 3; w++ {
        go worker(w, jobs, results)
    }
    
    for j := 1; j <= 9; j++ {
        jobs <- j
    }
    close(jobs)
    
    for a := 1; a <= 9; a++ {
        <-results
    }
}
```

### JSON

```json
{
  "project": "Yggdrasil",
  "version": "1.0.0",
  "description": "A modern blog system built with Rust and Dioxus",
  "dependencies": {
    "frontend": "Dioxus 0.7",
    "backend": "tokio-postgres",
    "database": "PostgreSQL 15"
  },
  "features": [
    "Markdown support",
    "Server-side rendering",
    "Real-time preview"
  ]
}
```

### SQL

```sql
-- 获取已发布的文章列表
SELECT 
    p.id,
    p.title,
    p.slug,
    p.summary,
    p.published_at,
    u.username as author
FROM posts p
JOIN users u ON p.author_id = u.id
WHERE p.status = 'published'
  AND p.deleted_at IS NULL
ORDER BY p.published_at DESC
LIMIT 20;
```

### Bash

```bash
#!/bin/bash

# 脚本：备份数据库
echo "开始备份..."

BACKUP_DIR="./backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
FILENAME="backup_${TIMESTAMP}.sql"

mkdir -p "$BACKUP_DIR"
pg_dump "$DATABASE_URL" > "$BACKUP_DIR/$FILENAME"

if [ $? -eq 0 ]; then
    echo "✅ 备份成功: $FILENAME"
    ls -lh "$BACKUP_DIR/$FILENAME"
else
    echo "❌ 备份失败"
    exit 1
fi
```

### 行内代码高亮

你也可以在段落中使用行内代码，比如 `git status`、`:wq`、`&lt;Route&gt;`、`&lt;Suspense&gt;`。

## 表格

Markdown 表格使用管道符 `|` 和连字符 `-` 构建：

### 基础表格

| 语言 | 类型 | 内存管理 | 并发模型 | 适用场景 |
|------|------|----------|----------|---------|
| Rust | 系统级 | 所有权系统 | Fearless Concurrency | 高性能系统、嵌入式 |
| Go | 系统级 | 垃圾回收 | Goroutine + Channel | 云原生、微服务 |
| Python | 高级动态 | 垃圾回收 (GIL) | 多进程/异步IO | 数据科学、Web开发 |
| JavaScript | 高级动态 | 垃圾回收 | 事件循环 + Promise | Web前端、Node.js |
| C++ | 系统级 | 手动/RAII | 线程 + 锁 | 游戏引擎、高频交易 |
| Zig | 系统级 | 显式分配 | 线程 | 系统工具、嵌入式 |

### 对齐方式

| 左对齐 | 居中对齐 | 右对齐 |
|:-------|:-------:|-------:|
| 内容 1 | 内容 2 | 内容 3 |
| A | B | 100 |
| 长文本示例 | 居中显示 | 999.99 |

## 水平线

水平线用于分隔文章的不同部分：

---

***

___

## 链接

Markdown 支持多种链接格式：

### 行内链接

[GitHub](https://github.com) - 世界上最流行的代码托管平台。

[相对链接](/) - 链接到网站首页。

[带标题的链接](https://example.com "示例网站") - 鼠标悬停显示标题。

### 引用式链接

[Google][google-link] 和 [Bing][bing-link] 是两大搜索引擎。

[google-link]: https://google.com "Google 搜索"
[bing-link]: https://bing.com "Bing 搜索"

## 图片

![Markdown Logo](https://markdown-here.com/img/icon256.png)

## HTML 内嵌

某些场景下，你可能需要直接使用 HTML：

<div style="padding: 1em; background: #f8f9fa; border-radius: 8px; border-left: 4px solid #007bff;">
  <p><strong>提示：</strong> 这是使用 HTML 创建的自定义样式块。当 Markdown 的表达能力不足时，可以直接嵌入 HTML。但请注意，这会降低内容的可移植性。</p>
</div>

## 特殊字符

Markdown 和 HTML 实体：

- 版权符号：&copy; 2024 Yggdrasil
- 注册商标：Markdown&reg;
- 商标符号：GitHub&trade;
- 长破折号：这是&mdash;一个长破折号
- 短破折号：这是&ndash;一个短破折号
- 省略号：等等&hellip;

## 脚注

脚注是学术写作中常用的功能[^1]。你可以在同一条注释中引用多个脚注[^2][^3]。

[^1]: 这是第一个脚注的内容。脚注可以包含多行文字和格式。
[^2]: 脚注通常用于提供补充说明或引用来源。
[^3]: Markdown 的脚注语法在不同的渲染引擎中支持程度不同。

## 总结

本文测试了 Markdown 的完整语法集：

| 语法元素 | 支持情况 | 说明 |
|---------|---------|------|
| 文本格式 | ✅ | 粗体、斜体、删除线、行内代码 |
| 标题 | ✅ | 六级标题支持 |
| 列表 | ✅ | 无序、有序、嵌套、任务列表 |
| 引用块 | ✅ | 支持嵌套引用 |
| 代码块 | ✅ | 语法高亮 |
| 表格 | ✅ | 对齐方式 |
| 水平线 | ✅ | 多种分隔符 |
| 链接 | ✅ | 行内和引用式 |
| 图片 | ✅ | 外部图片 |
| HTML | ✅ | 内嵌 HTML 块 |
| 脚注 | ⚠️ | 依赖渲染引擎支持 |

如果你的渲染引擎正确显示了以上内容，说明它对 Markdown 的支持非常完善！

---

*本文用于测试 Markdown 渲染引擎的兼容性。*
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '3 hours',
    NOW() - INTERVAL '3 hours',
    NOW() - INTERVAL '3 hours'
),
(
    1,
    'Rust 错误处理完全指南：Result、Option 与 thiserror',
    'rust-error-handling',
    'Rust 使用 Result 和 Option 类型来处理可能失败的操作和可能缺失的值，取代了传统的异常机制。本文深入讲解 Rust 的错误处理哲学和最佳实践。',
    $doc$
# Rust 错误处理完全指南：Result、Option 与 thiserror

Rust 没有异常机制（Exception）。对于习惯了 Java、Python 或 JavaScript 的开发者来说，这可能需要一些适应。但 Rust 的类型驱动错误处理——通过 `Result` 和 `Option` 类型——实际上是一种更健壮、更明确的设计。

本文将深入探讨 Rust 的错误处理哲学，从基础的 `Option` 和 `Result` 到自定义错误类型，再到 `?` 操作符和第三方库 `thiserror`。

## 为什么 Rust 没有异常？

传统的异常机制存在几个问题：

1. **不可见的控制流**：异常可以穿透任意层级的调用栈，使得代码路径难以追踪
2. **类型安全缺失**：编译器无法检查你是否处理了所有可能的异常
3. **性能开销**：异常处理通常需要运行时维护额外的栈信息

Rust 的解决方案是**将错误作为值返回**。如果一个函数可能失败，它的返回类型就明确地表示这一点。编译器会强迫你处理这个错误，否则代码无法通过编译。

## Option 类型：处理可能缺失的值

`Option<T>` 是 Rust 中表示"可能有值，也可能没有"的类型：

```rust
enum Option<T> {
    Some(T),  // 有值
    None,     // 无值
}
```

### 基础使用

```rust
fn find_char(s: &str, c: char) -> Option<usize> {
    s.find(c)  // 如果找到返回 Some(index)，否则返回 None
}

fn main() {
    // 使用 match 处理 Option
    match find_char("hello", 'e') {
        Some(index) => println!("✅ 找到字符在位置 {}", index),
        None => println!("❌ 未找到字符"),
    }
    
    // 使用 if let 简化（只关心 Some 的情况）
    if let Some(index) = find_char("hello", 'l') {
        println!("🎯 在位置 {}", index);
    }
    
    // while let：循环处理
    let mut text = "hello";
    while let Some(c) = text.chars().next() {
        println!("字符: {}", c);
        text = &text[1..];
    }
}
```

### Option 的组合子

`Option` 提供了丰富的组合子方法来链式处理：

```rust
fn main() {
    let maybe_number: Option<i32> = Some(5);
    
    // map：对 Some 中的值进行转换
    let doubled = maybe_number.map(|n| n * 2); // Some(10)
    
    // and_then：链式调用返回 Option 的函数
    let result = maybe_number
        .and_then(|n| if n > 3 { Some(n) } else { None })
        .map(|n| n * 2);
    
    // unwrap_or：提供默认值
    let value = maybe_number.unwrap_or(0); // 5
    let empty = None::<i32>.unwrap_or(0); // 0
    
    // unwrap_or_else：懒加载默认值
    let lazy = None::<i32>.unwrap_or_else(|| expensive_computation());
    
    // ok_or：将 Option 转换为 Result
    let result: Result<i32, &str> = maybe_number.ok_or("值为空");
}
```

## Result 类型：处理可能失败的操作

`Result<T, E>` 是 Rust 中表示操作可能失败的核心类型：

```rust
enum Result<T, E> {
    Ok(T),   // 成功
    Err(E),  // 失败，携带错误信息
}
```

### 基础使用

```rust
use std::fs::File;
use std::io::{self, Read};

fn read_username_from_file() -> Result<String, io::Error> {
    let mut file = File::open("hello.txt")?;
    let mut username = String::new();
    file.read_to_string(&mut username)?;
    Ok(username)
}
```

### ? 操作符：错误传播

`?` 操作符是 Rust 错误处理中最常用的语法糖。它在 `Result` 为 `Ok` 时解包值，在 `Err` 时提前返回错误：

```rust
fn read_config() -> Result<Config, io::Error> {
    let content = std::fs::read_to_string("config.json")?;  // 如果失败，直接返回 Err
    let config: Config = serde_json::from_str(&content)?;     // 如果失败，直接返回 Err
    Ok(config)
}
```

`?` 操作符的强大之处在于它可以自动进行错误类型的转换。只要当前函数的返回错误类型实现了 `From<E>`，就可以使用 `?`。

### Result 的组合子

与 `Option` 类似，`Result` 也有丰富的组合子：

```rust
fn main() {
    let result: Result<i32, &str> = Ok(42);
    
    // map：对 Ok 中的值进行转换
    let doubled = result.map(|n| n * 2); // Ok(84)
    
    // map_err：对 Err 中的值进行转换
    let with_prefix = Err("error").map_err(|e| format!("ERROR: {}", e));
    
    // and_then：链式调用
    let chained = Ok(5)
        .and_then(|n| if n > 0 { Ok(n * 2) } else { Err("负数") });
    
    // unwrap_or / unwrap_or_else
    let value = result.unwrap_or(0); // 42
    
    // expect：unwrap 的变体，可自定义 panic 消息
    let file = std::fs::read_to_string("config.txt")
        .expect("config.txt 必须存在");
}
```

## 自定义错误类型

在实际项目中，你可能需要定义自己的错误类型。Rust 提供了多种方式：

### 枚举错误类型

```rust
#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Parse(std::num::ParseIntError),
    InvalidInput(String),
    NotFound { resource: String, id: i64 },
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO 错误: {}", e),
            AppError::Parse(e) => write!(f, "解析错误: {}", e),
            AppError::InvalidInput(msg) => write!(f, "无效输入: {}", msg),
            AppError::NotFound { resource, id } => write!(f, "{} 不存在: id={}", resource, id),
        }
    }
}

impl std::error::Error for AppError {}

// 实现 From trait 以支持 ? 操作符
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(err: std::num::ParseIntError) -> Self {
        AppError::Parse(err)
    }
}
```

### 使用 thiserror 简化

手动实现错误类型很繁琐。[`thiserror`](https://docs.rs/thiserror) 是一个宏库，可以大大简化这个过程：

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("解析错误: {0}")]
    Parse(#[from] std::num::ParseIntError),
    
    #[error("无效输入: {0}")]
    InvalidInput(String),
    
    #[error("{resource} 不存在: id={id}")]
    NotFound { resource: String, id: i64 },
    
    #[error("数据库错误")]
    Database {
        #[from]
        source: sqlx::Error,
    },
}

// 使用
fn load_user(id: i64) -> Result<User, AppError> {
    let content = std::fs::read_to_string("users.json")?;  // 自动转换 io::Error
    let users: Vec<User> = serde_json::from_str(&content)?;
    
    users.into_iter()
        .find(|u| u.id == id)
        .ok_or_else(|| AppError::NotFound {
            resource: "User".to_string(),
            id,
        })
}
```

### anyhow：快速原型开发

如果你不需要精细的错误分类，可以使用 [`anyhow`](https://docs.rs/anyhow) 库：

```rust
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let config = std::fs::read_to_string("config.toml")
        .with_context(|| "无法读取配置文件")?;
    
    let settings: Settings = toml::from_str(&config)
        .context("配置文件格式错误")?;
    
    println!("{:?}", settings);
    Ok(())
}
```

## Option 与 Result 的转换

在实际代码中，经常需要在 `Option` 和 `Result` 之间转换：

```rust
// Option -> Result
let opt: Option<i32> = Some(5);
let res: Result<i32, &str> = opt.ok_or("值为空"); // Ok(5)
let res2 = opt.ok_or_else(|| format!("{} 为空", "value")); // Ok(5)

// Result -> Option
let res: Result<i32, &str> = Ok(5);
let opt = res.ok(); // Some(5)
let err_opt = res.err(); // None

// 在迭代中收集 Result
let numbers = vec!["1", "2", "3", "not_a_number"];
let parsed: Result<Vec<i32>, _> = numbers.iter()
    .map(|s| s.parse::<i32>())
    .collect(); // 如果有任何 Err，返回第一个 Err
```

## 错误处理最佳实践

| 场景 | 推荐方式 | 示例 |
|------|---------|------|
| 快速原型 | `anyhow::Result` | `fn main() -> anyhow::Result<()>` |
| 库开发 | `thiserror` | 定义精细的错误枚举 |
| 错误传播 | `?` 操作符 | `let x = may_fail()?;` |
| 提供默认值 | `unwrap_or` | `let x = opt.unwrap_or(0);` |
| 不可恢复错误 | `expect` | `let x = vec[0].expect("不能为空")` |
| 可选值链 | `and_then` + `map` | `opt.and_then(f).map(g)` |

## 总结

Rust 的错误处理机制强迫开发者显式处理所有错误路径，这看似繁琐，实际上带来了巨大的好处：

| 类型 | 表示 | 使用场景 |
|------|------|---------|
| `Option<T>` | `Some(T)` / `None` | 可能缺失的值 |
| `Result<T, E>` | `Ok(T)` / `Err(E)` | 可能失败的操作 |
| `?` 操作符 | 自动传播错误 | 函数内部错误处理 |
| `thiserror` | 宏生成的错误类型 | 库开发 |
| `anyhow` | 泛型错误类型 | 应用程序开发 |

Rust 的类型驱动错误处理是一种**以编译期检查换取运行时安全**的设计哲学。虽然需要写更多的错误处理代码，但换来的是程序在运行时几乎不会因为未处理的错误而崩溃。

> "在 Rust 中，如果代码通过了编译，那么它已经处理了所有可能的错误路径。" —— 这是 Rust 给予开发者的最大信心。

---

*本文首发于 Yggdrasil 博客*
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '2 hours',
    NOW() - INTERVAL '2 hours',
    NOW() - INTERVAL '2 hours'
),
(
    1,
    'Python 数据类完全指南：dataclass、attrs 与 Pydantic',
    'python-dataclasses',
    'Python 3.7 引入的 dataclass 装饰器大大简化了类的定义。本文深入对比 dataclass、attrs 和 Pydantic，帮助你选择最合适的方案。',
    $doc$
# Python 数据类完全指南：dataclass、attrs 与 Pydantic

在 Python 中定义一个简单的数据容器类，传统方式需要编写大量样板代码：`__init__`、`__repr__`、`__eq__`……这些代码不仅冗余，而且容易出错。Python 3.7 引入的 **`dataclass`** 装饰器彻底解决了这个问题，让你可以用声明式的方式定义数据类。

本文将深入介绍 `dataclass`、`attrs` 和 `Pydantic` 三种数据类方案，帮助你根据场景选择最合适的工具。

## dataclass（标准库）

`dataclasses` 模块是 Python 3.7+ 的标准库，无需安装任何第三方包。

### 基础用法

```python
from dataclasses import dataclass, field
from typing import List, Optional

@dataclass
class Person:
    name: str
    age: int = 0
    email: Optional[str] = None
    hobbies: List[str] = field(default_factory=list)
    
    def greet(self) -> str:
        return f"Hello, I'm {self.name}!"
    
    def is_adult(self) -> bool:
        return self.age >= 18

# 使用
person = Person("Alice", 30, "alice@example.com", ["reading", "coding"])
print(person)
# Person(name='Alice', age=30, email='alice@example.com', hobbies=['reading', 'coding'])

print(person.greet())  # Hello, I'm Alice!
print(person == Person("Alice", 30))  # False（因为 email 和 hobbies 不同）
```

`@dataclass` 装饰器自动为你生成了 `__init__`、`__repr__`、`__eq__` 等方法。

### dataclass 参数

```python
@dataclass(init=True,      # 生成 __init__
           repr=True,      # 生成 __repr__
           eq=True,        # 生成 __eq__
           order=False,    # 生成比较方法 (__lt__, __le__, __gt__, __ge__)
           unsafe_hash=False,  # 生成 __hash__
           frozen=False,   # 不可变实例
           slots=False,    # 使用 __slots__ 节省内存
           kw_only=False)  # 关键字-only 参数
class Product:
    name: str
    price: float
    quantity: int = 0
```

### field() 函数

`field()` 提供了更精细的控制：

```python
from dataclasses import dataclass, field

@dataclass
class User:
    id: int = field(init=False)  # 不通过 __init__ 传入
    name: str
    created_at: str = field(default_factory=lambda: datetime.now().isoformat())
    password: str = field(repr=False)  # 不在 __repr__ 中显示
    
    def __post_init__(self):
        self.id = hash(self.name) % 10000

user = User("alice", password="secret123")
print(user)
# User(id=1234, name='alice', created_at='2024-...')
# 注意：password 不在 repr 中
```

### frozen dataclass（不可变数据类）

```python
from dataclasses import dataclass

@dataclass(frozen=True)
class Point:
    x: float
    y: float
    
    def distance_from_origin(self) -> float:
        return (self.x ** 2 + self.y ** 2) ** 0.5

p = Point(3.0, 4.0)
print(p.distance_from_origin())  # 5.0
# p.x = 5.0  # FrozenInstanceError: cannot assign to field 'x'
```

不可变数据类天然是线程安全的，可以作为字典的键使用。

### 继承

```python
@dataclass
class Employee(Person):
    employee_id: str
    department: str = "Engineering"
    salary: float = field(default=0.0, repr=False)

emp = Employee("Bob", 25, None, [], "E001", "Product")
print(emp)
```

## attrs（第三方库）

[`attrs`](https://www.attrs.org/) 是比 `dataclass` 更早出现的第三方库，功能更强大。

```python
import attr

@attr.s(auto_attribs=True)
class Vehicle:
    wheels: int = 4
    color: str = "red"
    brand: str = attr.ib(default="Unknown", validator=attr.validators.instance_of(str))
    
    def describe(self) -> str:
        return f"A {self.color} {self.brand} with {self.wheels} wheels"

v = Vehicle(wheels=2, color="blue", brand="Yamaha")
print(v.describe())  # A blue Yamaha with 2 wheels
```

### attrs 的优势

| 特性 | dataclass | attrs | 说明 |
|------|-----------|-------|------|
| 标准库 | ✅ | ❌ | dataclass 无需安装 |
| 验证器 | 有限 | 强大 | attrs 内置多种验证器 |
| 转换器 | 无 | 有 | attrs 支持类型转换 |
| 性能 | 好 | 更好 | attrs 生成的代码更优化 |
| 序列化 | 需手动 | 内置 | attrs 支持多种格式 |
| 元数据 | 有限 | 丰富 | attrs 的 metadata 系统 |

### attrs 验证器示例

```python
import attr

@attr.s
class User:
    name: str = attr.ib(validator=attr.validators.instance_of(str))
    age: int = attr.ib(
        validator=[
            attr.validators.instance_of(int),
            attr.validators.ge(0),  # 大于等于 0
            attr.validators.le(150) # 小于等于 150
        ]
    )
    email: str = attr.ib(
        validator=attr.validators.matches_re(r"^[\w\.-]+@[\w\.-]+\.\w+$")
    )

# 使用
try:
    user = User("Alice", -5, "alice@example.com")
except ValueError as e:
    print(f"验证失败: {e}")
```

## Pydantic：数据验证与序列化

[`Pydantic`](https://docs.pydantic.dev/) 是目前 Python 生态系统中最流行的数据验证库，基于类型提示自动进行数据验证和转换。

### 基础用法

```python
from pydantic import BaseModel, Field, EmailStr, validator
from typing import List, Optional
from datetime import datetime

class User(BaseModel):
    id: int
    name: str = Field(..., min_length=1, max_length=100)
    email: EmailStr  # 自动验证邮箱格式
    age: int = Field(..., ge=0, le=150)
    is_active: bool = True
    tags: List[str] = []
    created_at: Optional[datetime] = None
    
    @validator('name')
    def name_must_not_be_empty(cls, v):
        if not v.strip():
            raise ValueError('Name must not be empty')
        return v.strip()

# 自动验证和转换
user = User(id=1, name="Alice", email="alice@example.com", age=30)
print(user)
# id=1 name='Alice' email='alice@example.com' age=30 is_active=True tags=[] created_at=None

# 从字典创建（自动转换类型）
user_dict = {"id": "2", "name": "Bob", "email": "bob@test.com", "age": "25"}
user2 = User(**user_dict)
print(user2.age)  # 25 (自动从 str 转换为 int)

# JSON 序列化
print(user.json())
# {"id": 1, "name": "Alice", "email": "alice@example.com", ...}
```

### Pydantic 配置

```python
from pydantic import BaseModel, Field

class Config:
    """Pydantic 配置选项"""
    pass

class User(BaseModel):
    class Config:
        # 允许从 ORM 对象创建
        orm_mode = True
        # 字段别名（如 JSON 中的 snake_case 映射到 camelCase）
        alias_generator = lambda x: x.lower()
        # 验证赋值
        validate_assignment = True
        # 错误信息使用中文
        error_msg_templates = {
            'value_error.missing': '字段必填',
        }
    
    name: str
    age: int
```

## 三者对比

| 特性 | dataclass | attrs | Pydantic |
|------|-----------|-------|----------|
| 安装 | 标准库 | `pip install attrs` | `pip install pydantic` |
| 类型转换 | ❌ | 部分 | ✅ 自动 |
| 数据验证 | ❌ | ✅ 内置 | ✅ 强大 |
| JSON 序列化 | ❌ | ✅ | ✅ 内置 |
| ORM 集成 | ❌ | ❌ | ✅ SQLAlchemy |
| 性能 | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐ |
| 学习曲线 | 低 | 中 | 低 |
| 适用场景 | 简单数据结构 | 复杂验证逻辑 | API 开发、配置管理 |

## 如何选择？

1. **简单数据类**：使用 `dataclass`（标准库，零依赖）
2. **需要复杂验证**：使用 `attrs`（验证器、转换器更强大）
3. **Web/API 开发**：使用 `Pydantic`（自动验证、JSON 序列化、FastAPI 原生支持）
4. **配置管理**：使用 `Pydantic`（环境变量加载、类型转换）
5. **ORM 模型**：使用 `Pydantic`（`orm_mode` 支持）

## 总结

Python 的数据类生态系统已经非常成熟：

- **`dataclass`**：简单、标准库、适合大多数场景
- **`attrs`**：功能强大、验证丰富、适合复杂业务逻辑
- **`Pydantic`**：验证 + 序列化 + ORM、Web 开发首选

三者并非互斥。你可以在项目中根据场景灵活选择。例如，内部数据模型使用 `dataclass`，API 接口使用 `Pydantic`，核心业务实体使用 `attrs`。

> **最佳实践**：不要过度设计。如果 `dataclass` 能满足需求，就不要引入额外的依赖。

---

*本文首发于 Yggdrasil 博客*
$doc$,
    NULL,
    'draft',
    NULL,
    NOW(),
    NOW()
),
(
    1,
    'C 语言指针详解',
    'c-pointers-explained',
    '深入理解 C 语言指针的本质，从基础概念到高级用法，全面掌握指针与数组、函数指针、多级指针、内存管理等核心技术，避免常见的指针陷阱。',
    $doc$
# C 语言指针详解：从基础到精通

指针是 C 语言最核心的特性之一，也是最令初学者困惑的概念。理解指针的本质，是掌握 C 语言的关键一步。本文将从基础概念出发，逐步深入探讨指针的各种高级用法和常见陷阱。

## 指针的本质

### 什么是指针

指针是一个变量，其值为另一个变量的内存地址。每个变量在内存中都有一个唯一的地址，指针就是存储这个地址的特殊变量。

```c
#include <stdio.h>

int main() {
    int num = 42;
    int *ptr = &num;

    printf("Value of num: %d\n", num);
    printf("Address of num: %p\n", (void*)&num);
    printf("Value of ptr (address): %p\n", (void*)ptr);
    printf("Value pointed by ptr: %d\n", *ptr);

    return 0;
}
```

在这个例子中：

| 表达式 | 含义 |
|--------|------|
| `&num` | 取变量 num 的地址 |
| `int *ptr` | 声明一个指向 int 的指针 |
| `*ptr` | 解引用，获取指针指向的值 |
| `ptr = &num` | 将 num 的地址赋给指针 |

### 指针的大小

指针的大小取决于系统的架构，而不是指向的数据类型：

```c
#include <stdio.h>

int main() {
    int *ip;
    char *cp;
    double *dp;
    void *vp;

    printf("sizeof(int*):   %zu bytes\n", sizeof(ip));
    printf("sizeof(char*):  %zu bytes\n", sizeof(cp));
    printf("sizeof(double*):%zu bytes\n", sizeof(dp));
    printf("sizeof(void*):  %zu bytes\n", sizeof(vp));

    return 0;
}
```

在 64 位系统中，所有指针通常都是 8 字节（64 位）。

## 指针与数组

### 数组名的本质

数组名在大多数表达式中会退化为指向数组首元素的指针：

```c
#include <stdio.h>

int main() {
    int arr[5] = {10, 20, 30, 40, 50};
    int *ptr = arr;  // 等价于 &arr[0]

    // 以下四种写法等价
    printf("arr[2] = %d\n", arr[2]);
    printf("ptr[2] = %d\n", ptr[2]);
    printf("*(arr + 2) = %d\n", *(arr + 2));
    printf("*(ptr + 2) = %d\n", *(ptr + 2));

    return 0;
}
```

### 指针算术

指针算术会自动考虑数据类型的大小：

```c
#include <stdio.h>

int main() {
    int arr[5] = {10, 20, 30, 40, 50};
    int *ptr = arr;

    printf("ptr        = %p\n", (void*)ptr);
    printf("ptr + 1    = %p\n", (void*)(ptr + 1));
    printf("ptr + 2    = %p\n", (void*)(ptr + 2));

    // 差值计算的是元素个数，不是字节数
    printf("(ptr + 3) - ptr = %ld\n", (ptr + 3) - ptr);

    return 0;
}
```

> **注意**：指针算术只在指向数组元素时才有意义。对非数组对象的指针进行算术运算会导致未定义行为。

## 多级指针

### 指向指针的指针

```c
#include <stdio.h>

int main() {
    int num = 42;
    int *ptr1 = &num;
    int **ptr2 = &ptr1;
    int ***ptr3 = &ptr2;

    printf("num    = %d\n", num);
    printf("*ptr1  = %d\n", *ptr1);
    printf("**ptr2 = %d\n", **ptr2);
    printf("***ptr3 = %d\n", ***ptr3);

    // 修改值
    ***ptr3 = 100;
    printf("After modification: num = %d\n", num);

    return 0;
}
```

多级指针常用于动态二维数组和函数参数传递：

```c
#include <stdio.h>
#include <stdlib.h>

// 使用二级指针创建动态二维数组
int** create_matrix(int rows, int cols) {
    int **matrix = malloc(rows * sizeof(int*));
    for (int i = 0; i < rows; i++) {
        matrix[i] = malloc(cols * sizeof(int));
    }
    return matrix;
}

void free_matrix(int **matrix, int rows) {
    for (int i = 0; i < rows; i++) {
        free(matrix[i]);
    }
    free(matrix);
}
```

## 函数指针

### 基本语法

函数指针允许我们将函数作为参数传递，实现回调机制：

```c
#include <stdio.h>

// 函数指针类型定义
typedef int (*CompareFunc)(int, int);

int add(int a, int b) { return a + b; }
int subtract(int a, int b) { return a - b; }
int multiply(int a, int b) { return a * b; }

// 使用函数指针作为参数
int operate(int a, int b, CompareFunc op) {
    return op(a, b);
}

int main() {
    printf("add:      %d\n", operate(10, 5, add));
    printf("subtract: %d\n", operate(10, 5, subtract));
    printf("multiply: %d\n", operate(10, 5, multiply));

    return 0;
}
```

### 回调函数应用

```c
#include <stdio.h>

// 通用排序函数，使用回调比较
typedef int (*CompareFunc)(const void*, const void*);

void bubble_sort(void *arr, size_t n, size_t size, CompareFunc cmp) {
    char *base = arr;
    char temp[size];

    for (size_t i = 0; i < n - 1; i++) {
        for (size_t j = 0; j < n - i - 1; j++) {
            if (cmp(base + j * size, base + (j + 1) * size) > 0) {
                // 交换
                memcpy(temp, base + j * size, size);
                memcpy(base + j * size, base + (j + 1) * size, size);
                memcpy(base + (j + 1) * size, temp, size);
            }
        }
    }
}

int int_cmp(const void *a, const void *b) {
    return (*(int*)a - *(int*)b);
}

int main() {
    int arr[] = {64, 34, 25, 12, 22, 11, 90};
    size_t n = sizeof(arr) / sizeof(arr[0]);

    bubble_sort(arr, n, sizeof(int), int_cmp);

    printf("Sorted array: ");
    for (size_t i = 0; i < n; i++) {
        printf("%d ", arr[i]);
    }
    printf("\n");

    return 0;
}
```

## void 指针

`void*` 是一种通用指针类型，可以指向任何数据类型：

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// 通用的内存拷贝函数
void* my_memcpy(void *dest, const void *src, size_t n) {
    char *d = dest;
    const char *s = src;
    while (n--) {
        *d++ = *s++;
    }
    return dest;
}

// 泛型交换函数
void swap(void *a, void *b, size_t size) {
    char temp[size];
    memcpy(temp, a, size);
    memcpy(a, b, size);
    memcpy(b, temp, size);
}

int main() {
    int x = 10, y = 20;
    swap(&x, &y, sizeof(int));
    printf("x = %d, y = %d\n", x, y);

    double a = 1.5, b = 2.5;
    swap(&a, &b, sizeof(double));
    printf("a = %.1f, b = %.1f\n", a, b);

    return 0;
}
```

## const 与指针

`const` 与指针的组合有四种情况，每种含义不同：

| 声明 | 读法 | 含义 |
|------|------|------|
| `int* ptr` | 指向 int 的指针 | 指针和值都可变 |
| `const int* ptr` | 指向常量的指针 | 值不可变，指针可变 |
| `int* const ptr` | 常量指针 | 值可变，指针不可变 |
| `const int* const ptr` | 指向常量的常量指针 | 都不可变 |

```c
#include <stdio.h>

int main() {
    int a = 10, b = 20;

    // 1. 指向常量的指针（常量指针）
    const int *ptr1 = &a;
    // *ptr1 = 30;  // 错误！不能通过 ptr1 修改值
    ptr1 = &b;     // 正确，可以修改指针指向

    // 2. 常量指针
    int *const ptr2 = &a;
    *ptr2 = 30;    // 正确，可以修改值
    // ptr2 = &b;  // 错误！不能修改指针指向

    // 3. 指向常量的常量指针
    const int *const ptr3 = &a;
    // *ptr3 = 40;  // 错误！
    // ptr3 = &b;   // 错误！

    printf("a = %d\n", a);

    return 0;
}
```

## 内存布局与对齐

```c
#include <stdio.h>

struct Example {
    char c;
    int i;
    char d;
};

struct PackedExample {
    char c;
    int i;
    char d;
} __attribute__((packed));

int main() {
    printf("sizeof(Example):      %zu\n", sizeof(struct Example));
    printf("sizeof(PackedExample): %zu\n", sizeof(struct PackedExample));

    struct Example ex;
    printf("Address of c: %p\n", (void*)&ex.c);
    printf("Address of i: %p\n", (void*)&ex.i);
    printf("Address of d: %p\n", (void*)&ex.d);

    return 0;
}
```

## 常见指针陷阱

### 1. 未初始化的指针

```c
int *ptr;       // 野指针，指向随机地址
*ptr = 10;      // 未定义行为！可能导致程序崩溃
```

### 2. 内存泄漏

```c
void leak_example() {
    int *ptr = malloc(sizeof(int) * 100);
    // ... 使用 ptr
    // 忘记 free(ptr) —— 内存泄漏！
}
```

### 3. 悬空指针

```c
int* dangling_pointer() {
    int local = 42;
    return &local;  // 危险！返回局部变量的地址
}
```

### 4. 数组越界

```c
int arr[5] = {1, 2, 3, 4, 5};
int *ptr = arr;
ptr[5] = 10;  // 越界访问！未定义行为
```

## 总结

指针是 C 语言强大而灵活的工具，掌握指针需要理解：

1. **内存模型**：理解变量在内存中的存储方式
2. **类型系统**：指针类型决定了指针算术的步长
3. **生命周期**：确保指针始终指向有效的内存
4. **所有权**：谁分配、谁释放，避免内存泄漏

> "C 语言让你可以做出所有你想要的错误，包括那些你可能没想到的。" —— 对指针最好的描述

通过本文的学习，你应该能够自信地使用指针，并避免常见的陷阱。记住，**理解指针的本质是理解内存**，这是成为优秀 C 程序员的关键。
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '1 hour',
    NOW() - INTERVAL '1 hour',
    NOW() - INTERVAL '1 hour'
),
(
    1,
    'Elixir 并发与容错',
    'elixir-concurrency',
    '深入探索 Elixir 的并发模型和容错机制，从 OTP 进程到 Supervisor 监督树，理解 Actor 模型和 Let it crash 哲学，构建高可用的分布式系统。',
    $doc$
# Elixir 并发与容错：构建高可用系统

Elixir 构建在 Erlang VM（BEAM）之上，继承了 Erlang 强大的并发和容错能力。本文将深入探讨 Elixir 的并发模型、进程隔离、Supervisor 监督机制以及分布式系统的构建方法。

## 为什么选择 Elixir？

Elixir 不是另一种 Web 框架，而是一种全新的编程范式。它基于 **Actor 模型**，提供了轻量级进程和消息传递机制，让并发编程变得简单而安全。

> WhatsApp 使用 Erlang/OTP 处理每天超过 650 亿条消息，每台服务器维护超过 200 万个并发连接。这就是 BEAM 虚拟机的实力。

## Elixir 进程模型

### 轻量级进程

在 Elixir 中，"进程"不是操作系统进程，而是由 BEAM 虚拟机管理的轻量级执行单元：

```elixir
# 创建一个新进程
pid = spawn(fn ->
  IO.puts("Hello from a new process!")
  IO.puts("Process PID: #{inspect(self())}")
end)

IO.puts("Main process PID: #{inspect(self())}")
IO.puts("Spawned process PID: #{inspect(pid)}")
```

特点对比：

| 特性 | OS 进程 | Elixir 进程 |
|------|---------|-------------|
| 内存占用 | MB 级别 | ~300 字节 |
| 创建时间 | 毫秒级 | 微秒级 |
| 最大数量 | 数千 | 数百万 |
| 通信方式 | 共享内存 | 消息传递 |
| 调度 | 抢占式 | 协作式 |

### 进程隔离

每个 Elixir 进程都有自己独立的内存空间，进程之间不共享状态：

```elixir
defmodule Counter do
  def start(initial_value \\ 0) do
    spawn(fn -> loop(initial_value) end)
  end

  defp loop(current_value) do
    receive do
      {:get, caller} ->
        send(caller, {:value, current_value})
        loop(current_value)

      {:increment} ->
        loop(current_value + 1)

      {:decrement} ->
        loop(current_value - 1)

      {:add, amount} ->
        loop(current_value + amount)
    end
  end
end

# 使用示例
counter = Counter.start(10)

send(counter, {:increment})
send(counter, {:add, 5})
send(counter, {:get, self()})

receive do
  {:value, value} -> IO.puts("Current value: #{value}")
after
  1000 -> IO.puts("Timeout!")
end
```

## 消息传递机制

### 异步消息

Elixir 进程通过异步消息传递进行通信：

```elixir
defmodule Messenger do
  def send_message(to_pid, message) do
    send(to_pid, {self(), message})
  end

  def wait_for_reply(timeout \\\\ 5000) do
    receive do
      {_from, reply} -> {:ok, reply}
    after
      timeout -> {:error, :timeout}
    end
  end
end

# 创建接收进程
receiver = spawn(fn ->
  receive do
    {from, msg} ->
      IO.puts("Received: #{msg}")
      send(from, {self(), "Reply: #{msg}"})
  end
end)

Messenger.send_message(receiver, "Hello Elixir!")
case Messenger.wait_for_reply() do
  {:ok, reply} -> IO.puts("Got reply: #{reply}")
  {:error, reason} -> IO.puts("Failed: #{reason}")
end
```

### 消息邮箱

每个进程都有一个消息邮箱，消息按到达顺序排队：

```elixir
defmodule MessageProcessor do
  def process_messages do
    receive do
      message ->
        IO.puts("Processing: #{inspect(message)}")
        process_messages()
    end
  end
end

processor = spawn(&MessageProcessor.process_messages/0)

send(processor, :first)
send(processor, :second)
send(processor, {:complex, [1, 2, 3]})
```

## GenServer：状态机模式

GenServer 是 OTP 行为（Behaviour）之一，提供了一种标准化的方式来构建有状态的服务器进程：

```elixir
defmodule KeyValueStore do
  use GenServer

  # 客户端 API
  def start_link(initial_state \\\\ %{}) do
    GenServer.start_link(__MODULE__, initial_state, name: __MODULE__)
  end

  def get(key) do
    GenServer.call(__MODULE__, {:get, key})
  end

  def put(key, value) do
    GenServer.cast(__MODULE__, {:put, key, value})
  end

  def delete(key) do
    GenServer.call(__MODULE__, {:delete, key})
  end

  # 服务器回调
  @impl true
  def init(initial_state) do
    {:ok, initial_state}
  end

  @impl true
  def handle_call({:get, key}, _from, state) do
    {:reply, Map.get(state, key), state}
  end

  @impl true
  def handle_call({:delete, key}, _from, state) do
    {:reply, :ok, Map.delete(state, key)}
  end

  @impl true
  def handle_cast({:put, key, value}, state) do
    {:noreply, Map.put(state, key, value)}
  end
end

# 使用示例
{:ok, _pid} = KeyValueStore.start_link()
KeyValueStore.put(:name, "Elixir")
KeyValueStore.put(:version, "1.15")
IO.inspect(KeyValueStore.get(:name))
```

### GenServer 回调详解

| 回调函数 | 用途 | 返回值 |
|----------|------|--------|
| `init/1` | 初始化状态 | `{:ok, state}` |
| `handle_call/3` | 同步请求 | `{:reply, reply, state}` |
| `handle_cast/2` | 异步请求 | `{:noreply, state}` |
| `handle_info/2` | 处理普通消息 | `{:noreply, state}` |
| `terminate/2` | 清理资源 | 任意 |

## Supervisor 监督策略

### 容错哲学：Let it crash

Elixir 的核心哲学是 **Let it crash**。进程崩溃不应该影响整个系统，Supervisor 会负责重启失败的进程。

```elixir
defmodule MyApp.Supervisor do
  use Supervisor

  def start_link(init_arg) do
    Supervisor.start_link(__MODULE__, init_arg, name: __MODULE__)
  end

  @impl true
  def init(_init_arg) do
    children = [
      # 工作进程
      {KeyValueStore, %{}},
      # 另一个工作进程
      {MyApp.Worker, []},
      # 动态监督者
      {DynamicSupervisor, strategy: :one_for_one, name: MyApp.DynamicSupervisor}
    ]

    Supervisor.init(children, strategy: :one_for_all)
  end
end
```

### 监督策略对比

| 策略 | 说明 | 适用场景 |
|------|------|----------|
| `:one_for_one` | 只重启失败的子进程 | 独立的服务 |
| `:one_for_all` | 重启所有子进程 | 相互依赖的服务 |
| `:rest_for_one` | 重启失败进程及其后续进程 | 有启动顺序依赖 |
| `:simple_one_for_one` | 动态添加子进程 | 需要动态创建进程 |

### 重启策略

```elixir
defmodule MyApp.Worker do
  use GenServer

  def start_link(args) do
    GenServer.start_link(__MODULE__, args)
  end

  @impl true
  def init(args) do
    # 如果初始化失败，Supervisor 会根据重启策略处理
    case connect_to_database(args) do
      {:ok, conn} -> {:ok, conn}
      {:error, reason} -> {:stop, reason}
    end
  end

  defp connect_to_database(_args) do
    # 模拟数据库连接
    {:ok, %{} }
  end
end
```

## 容错机制实践

### 链接进程（Linking）

进程可以相互链接，一个进程崩溃会导致链接的进程也崩溃：

```elixir
defmodule LinkExample do
  def run do
    parent = self()

    child = spawn_link(fn ->
      receive do
        :crash -> raise "Simulated error"
        :normal -> IO.puts("Normal exit")
      end
    end)

    Process.flag(:trap_exit, true)

    send(child, :crash)

    receive do
      {:EXIT, ^child, reason} ->
        IO.puts("Child exited with reason: #{inspect(reason)}")
    end
  end
end
```

### 监控进程（Monitoring）

监控是单向的，被监控进程崩溃不会导致监控进程崩溃：

```elixir
defmodule MonitorExample do
  def run do
    target = spawn(fn ->
      Process.sleep(1000)
      exit(:normal)
    end)

    ref = Process.monitor(target)

    receive do
      {:DOWN, ^ref, :process, ^target, reason} ->
        IO.puts("Process down: #{inspect(reason)}")
    end
  end
end
```

### 选择策略

| 特性 | Link | Monitor |
|------|------|---------|
| 方向 | 双向 | 单向 |
| 影响 | 相互影响 | 仅通知 |
| 用途 | 构建 Supervision Tree | 观察进程状态 |
| 退出传播 | 是 | 否 |

## 分布式 Elixir

### 节点间通信

Elixir 进程可以在不同机器间透明通信：

```elixir
# 启动节点 1（机器 A）
# iex --sname node1@machine_a --cookie secret

# 启动节点 2（机器 B）
# iex --sname node2@machine_b --cookie secret

# 在 node2 上连接到 node1
Node.connect(:"node1@machine_a")

# 在 node2 上向 node1 发送消息
send({:some_process, :"node1@machine_a"}, :hello_from_node2)

# 在 node1 上接收
receive do
  msg -> IO.puts("Received: #{inspect(msg)}")
end
```

### 分布式任务

```elixir
defmodule DistributedTask do
  def run_on_all_nodes(task) do
    nodes = [Node.self() | Node.list()]

    Enum.map(nodes, fn node ->
      Task.Supervisor.async_nolink({MyApp.TaskSupervisor, node}, fn ->
        result = task.()
        {node, result}
      end)
    end)
    |> Enum.map(&Task.await/1)
  end
end

# 在所有节点上执行计算
results = DistributedTask.run_on_all_nodes(fn ->
  # 计算密集型任务
  Enum.sum(1..1_000_000)
end)

IO.inspect(results)
```

## 实际应用：构建容错计数器

```elixir
defmodule FaultTolerantCounter do
  use GenServer

  def start_link(_opts) do
    GenServer.start_link(__MODULE__, 0, name: __MODULE__)
  end

  def increment do
    GenServer.cast(__MODULE__, :increment)
  end

  def get_count do
    GenServer.call(__MODULE__, :get_count)
  end

  @impl true
  def init(state) do
    # 设置陷阱退出，处理链接进程的错误
    Process.flag(:trap_exit, true)
    {:ok, state}
  end

  @impl true
  def handle_cast(:increment, state) do
    {:noreply, state + 1}
  end

  @impl true
  def handle_call(:get_count, _from, state) do
    {:reply, state, state}
  end

  @impl true
  def handle_info({:EXIT, _pid, reason}, state) do
    IO.puts("Linked process exited: #{inspect(reason)}")
    {:noreply, state}
  end
end

# Supervisor 配置
defmodule CounterSupervisor do
  use Supervisor

  def start_link(init_arg) do
    Supervisor.start_link(__MODULE__, init_arg, name: __MODULE__)
  end

  @impl true
  def init(_init_arg) do
    children = [
      {FaultTolerantCounter, []}
    ]

    # 使用 transient 重启策略，只在异常退出时重启
    Supervisor.init(children,
      strategy: :one_for_one,
      max_restarts: 5,
      max_seconds: 10
    )
  end
end
```

## 总结

Elixir 的并发和容错能力源于其独特的设计哲学：

1. **隔离性**：进程完全隔离，一个崩溃不影响其他
2. **监督**：Supervisor 自动重启失败进程
3. **消息传递**：避免共享状态带来的复杂性
4. **热升级**：不停机更新运行中的系统
5. **分布式透明**：跨节点通信与本地通信 API 一致

> "Let it crash" 不是忽视错误，而是设计系统时的基本假设 —— 组件会失败，但系统必须继续运行。

通过掌握这些概念，你可以构建出真正高可用、容错的分布式系统，充分利用多核 CPU 和集群环境。
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '30 minutes',
    NOW() - INTERVAL '30 minutes',
    NOW() - INTERVAL '30 minutes'
),
(
    1,
    'Kotlin 协程与 Flow',
    'kotlin-coroutines-flow',
    '全面解析 Kotlin 协程和 Flow，从基础概念到高级用法，深入理解结构化并发、冷流热流、状态管理，以及与 RxJava 的对比，掌握现代 Kotlin 异步编程。',
    $doc$
# Kotlin 协程与 Flow：现代异步编程指南

Kotlin 协程（Coroutines）彻底改变了 Android 和后端开发中的异步编程方式。本文将从基础概念出发，深入探讨协程的各种用法、Flow 响应式流，以及与 RxJava 的对比。

## 为什么选择协程？

传统的回调式异步编程导致代码难以阅读和维护，俗称"回调地狱"。Kotlin 协程提供了**挂起函数（Suspending Functions）**，让异步代码像同步代码一样简洁。

> 协程不是线程，线程也不是协程。协程是**可挂起的计算**，可以在单个线程上运行多个协程。

## 协程基础

### 启动协程

Kotlin 提供了三种启动协程的方式：

```kotlin
import kotlinx.coroutines.*

fun main() = runBlocking {
    // 1. launch - 启动新协程，不返回结果
    val job = launch {
        delay(1000L)
        println("World!")
    }

    // 2. async - 启动新协程，返回 Deferred（ future/promise）
    val deferred = async {
        delay(500L)
        "Hello"
    }

    // 3. runBlocking - 阻塞当前线程等待协程完成
    println("${deferred.await()}, ${job.join()}")
}
```

| 构建器 | 返回类型 | 用途 |
|--------|----------|------|
| `launch` | `Job` | 启动"开火即忘"的协程 |
| `async` | `Deferred<T>` | 启动返回结果的协程 |
| `runBlocking` | `T` | 桥接阻塞和非阻塞代码 |

### 挂起函数

挂起函数是协程的核心，可以在不阻塞线程的情况下暂停执行：

```kotlin
import kotlinx.coroutines.*

suspend fun fetchUserData(userId: String): User {
    // 模拟网络请求
    delay(1000) // 挂起 1 秒，不阻塞线程
    return User(userId, "John Doe", "john@example.com")
}

suspend fun fetchUserOrders(userId: String): List<Order> {
    delay(800)
    return listOf(
        Order("1", 99.99),
        Order("2", 149.99)
    )
}

// 组合挂起函数
suspend fun getUserProfile(userId: String): UserProfile {
    val user = fetchUserData(userId)
    val orders = fetchUserOrders(userId)
    return UserProfile(user, orders)
}
```

### CoroutineScope 和上下文

```kotlin
import kotlinx.coroutines.*
import kotlin.coroutines.CoroutineContext

// 自定义 Scope
class Activity : CoroutineScope {
    private val job = Job()

    override val coroutineContext: CoroutineContext
        get() = Dispatchers.Main + job

    fun destroy() {
        job.cancel() // 取消所有子协程
    }

    fun loadData() {
        launch {
            try {
                val data = fetchData()
                updateUI(data)
            } catch (e: CancellationException) {
                println("Coroutine cancelled")
            }
        }
    }

    private suspend fun fetchData(): String {
        delay(2000)
        return "Data loaded"
    }

    private fun updateUI(data: String) {
        println("UI updated: $data")
    }
}
```

## 结构化并发

### 父子关系

协程形成树形结构，父协程取消会自动取消所有子协程：

```kotlin
import kotlinx.coroutines.*

fun main() = runBlocking {
    val parentJob = launch {
        // 子协程 1
        launch {
            repeat(10) { i ->
                println("Child 1: $i")
                delay(100)
            }
        }

        // 子协程 2
        launch {
            repeat(10) { i ->
                println("Child 2: $i")
                delay(150)
            }
        }

        delay(300)
        println("Parent: Cancelling...")
    }

    parentJob.join()
    println("All coroutines completed")
}
```

### SupervisorJob

当不希望子协程的失败影响其他子协程时，使用 SupervisorJob：

```kotlin
import kotlinx.coroutines.*

fun main() = runBlocking {
    val supervisor = SupervisorJob()

    with(CoroutineScope(coroutineContext + supervisor)) {
        // 第一个子协程 - 会失败
        val job1 = launch {
            delay(100)
            throw RuntimeException("Oops!")
        }

        // 第二个子协程 - 不受影响继续运行
        val job2 = launch {
            repeat(5) { i ->
                println("Job2: $i")
                delay(100)
            }
        }

        joinAll(job1, job2)
    }
}
```

## Flow：响应式流

### 冷流（Cold Flow）

Flow 是 Kotlin 的响应式流实现，采用**冷流**模式 —— 数据在订阅时才产生：

```kotlin
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*

fun simpleFlow(): Flow<Int> = flow {
    println("Flow started")
    for (i in 1..3) {
        delay(100)
        emit(i) // 发射值
    }
}

fun main() = runBlocking {
    val flow = simpleFlow()

    println("First collection:")
    flow.collect { value ->
        println("Received: $value")
    }

    println("\nSecond collection:")
    flow.collect { value ->
        println("Received again: $value")
    }
}
```

### 操作符

Flow 提供了丰富的操作符，类似于 RxJava：

```kotlin
import kotlinx.coroutines.flow.*

fun processNumbers(): Flow<Int> = flow {
    (1..10).forEach { emit(it) }
}

suspend fun demonstrateOperators() {
    val result = processNumbers()
        .filter { it % 2 == 0 }      // 过滤偶数
        .map { it * it }              // 平方
        .take(3)                      // 取前 3 个
        .onEach { println("Processing: $it") }
        .toList()                     // 收集为列表

    println("Result: $result")
}
```

| 操作符类别 | 示例 | 说明 |
|-----------|------|------|
| 转换 | `map`, `transform` | 转换每个元素 |
| 过滤 | `filter`, `take`, `drop` | 筛选元素 |
| 组合 | `zip`, `combine`, `merge` | 组合多个 Flow |
| 错误处理 | `catch`, `retry` | 处理异常 |
| 终端 | `collect`, `reduce`, `fold` | 收集结果 |

### StateFlow 和 SharedFlow

StateFlow 和 SharedFlow 是**热流**，适合状态管理和事件分发：

```kotlin
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*

class NewsViewModel {
    // StateFlow - 总是有值，适合 UI 状态
    private val _uiState = MutableStateFlow<UiState>(UiState.Loading)
    val uiState: StateFlow<UiState> = _uiState.asStateFlow()

    // SharedFlow - 适合一次性事件
    private val _events = MutableSharedFlow<String>()
    val events: SharedFlow<String> = _events.asSharedFlow()

    fun loadNews() {
        viewModelScope.launch {
            _uiState.value = UiState.Loading

            try {
                val news = repository.fetchNews()
                _uiState.value = UiState.Success(news)
            } catch (e: Exception) {
                _uiState.value = UiState.Error(e.message ?: "Unknown error")
                _events.emit("Failed to load news")
            }
        }
    }
}

sealed class UiState {
    object Loading : UiState()
    data class Success(val data: List<News>) : UiState()
    data class Error(val message: String) : UiState()
}
```

## 异常处理

### try-catch 在协程中

```kotlin
import kotlinx.coroutines.*

fun main() = runBlocking {
    val job = launch {
        try {
            riskyOperation()
        } catch (e: Exception) {
            println("Caught: ${e.message}")
        }
    }

    job.join()
}

suspend fun riskyOperation() {
    delay(100)
    throw RuntimeException("Something went wrong")
}
```

### CoroutineExceptionHandler

```kotlin
import kotlinx.coroutines.*

fun main() = runBlocking {
    val handler = CoroutineExceptionHandler { _, exception ->
        println("Caught $exception")
    }

    val job = GlobalScope.launch(handler) {
        throw AssertionError("My Error")
    }

    val deferred = GlobalScope.async(handler) {
        throw ArithmeticException()
    }

    joinAll(job, deferred)
}
```

### Flow 异常处理

```kotlin
import kotlinx.coroutines.flow.*

fun safeFlow(): Flow<Int> = flow {
    emit(1)
    emit(2)
    throw RuntimeException("Error!")
}.catch { e ->
    println("Caught: ${e.message}")
    emit(-1) // 发射默认值
}.onCompletion { cause ->
    if (cause != null) {
        println("Flow completed with error")
    } else {
        println("Flow completed successfully")
    }
}
```

## 与 RxJava 对比

| 特性 | Kotlin Flow | RxJava |
|------|-------------|---------|
| 学习曲线 | 平缓 | 陡峭 |
| 内存开销 | 低 | 较高 |
| Android 支持 | 原生（官方推荐） | 需要额外依赖 |
| 线程切换 | `withContext` | `subscribeOn`/`observeOn` |
| 背压支持 | `buffer`, `conflate` | 内置 Backpressure |
| 取消机制 | 结构化并发 | Disposable |
| 冷/热流 | 明确区分 | 较模糊 |

### 从 RxJava 迁移示例

```kotlin
// RxJava 方式
Observable.fromIterable(users)
    .subscribeOn(Schedulers.io())
    .observeOn(AndroidSchedulers.mainThread())
    .map { it.name }
    .subscribe { println(it) }

// Flow 方式
users.asFlow()
    .flowOn(Dispatchers.IO)
    .map { it.name }
    .collect { println(it) }
```

## 实际应用：MVVM 架构

```kotlin
import androidx.lifecycle.*
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch

class UserViewModel(
    private val userRepository: UserRepository
) : ViewModel() {

    private val _searchQuery = MutableStateFlow("")
    val searchQuery: StateFlow<String> = _searchQuery.asStateFlow()

    // 搜索用户，自动去抖
    val users: StateFlow<List<User>> = _searchQuery
        .debounce(300) // 等待 300ms 无输入才搜索
        .flatMapLatest { query ->
            if (query.isEmpty()) {
                flowOf(emptyList())
            } else {
                userRepository.searchUsers(query)
            }
        }
        .stateIn(
            scope = viewModelScope,
            started = SharingStarted.WhileSubscribed(5000),
            initialValue = emptyList()
        )

    fun onSearchQueryChange(query: String) {
        _searchQuery.value = query
    }

    fun refreshUsers() {
        viewModelScope.launch {
            try {
                userRepository.refreshUsers()
            } catch (e: Exception) {
                // 处理错误
            }
        }
    }
}
```

## 总结

Kotlin 协程和 Flow 提供了一套完整的异步编程解决方案：

1. **轻量级**：协程比线程更轻量，单线程可运行数千协程
2. **结构化并发**：自动管理协程生命周期，避免泄漏
3. **Flow 响应式**：声明式处理数据流，操作符丰富
4. **异常安全**：完善的异常处理机制
5. **与 Android 深度集成**：ViewModelScope、LifecycleScope 等

> 掌握协程和 Flow，是成为现代 Kotlin 开发者的必备技能。

通过本文的学习，你应该能够在实际项目中熟练使用协程进行网络请求、数据库操作、UI 更新等异步任务，并使用 Flow 构建响应式的数据流。
$doc$,
    NULL,
    'published',
    NOW() - INTERVAL '15 minutes',
    NOW() - INTERVAL '15 minutes',
    NOW() - INTERVAL '15 minutes'
),
(
    1,
    'Swift 现代语法速览',
    'swift-modern-syntax',
    '全面梳理 Swift 现代语法特性，从可选类型到属性包装器，深入理解值类型与引用类型、协议扩展、泛型和错误处理，掌握 Swift 编程的核心概念和最佳实践。',
    $doc$
# Swift 现代语法速览

Swift 是 Apple 推出的现代编程语言，结合了 C 和 Objective-C 的优点，同时引入了函数式编程和类型安全的特性。本文将全面梳理 Swift 的核心语法，帮助你快速掌握这门语言。

## 为什么选择 Swift？

Swift 的设计目标是安全、快速、表达力强。它消除了 C 语言家族中许多不安全的特性，同时提供了现代化的语法和强大的类型系统。

> Swift 结合了编译型语言的性能和脚本语言的交互性，是现代 Apple 平台开发的首选语言。

## 基础语法

### 常量与变量

```swift
// 常量 - 不可变
let pi = 3.14159
let greeting = "Hello, Swift!"

// 变量 - 可变
var counter = 0
counter += 1

// 类型注解（通常可以省略，编译器会自动推断）
let explicitDouble: Double = 70.0
let implicitDouble = 70.0 // 同样是 Double

// 多行字符串
let multiline = """
    Swift is a powerful and intuitive
    programming language for iOS, iPadOS,
    macOS, watchOS, and tvOS.
    """
```

### 基本数据类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `Int` | 整数 | `let age: Int = 25` |
| `Double` | 双精度浮点 | `let price: Double = 19.99` |
| `String` | 字符串 | `let name = "Swift"` |
| `Bool` | 布尔 | `let isActive = true` |
| `Array` | 数组 | `let numbers = [1, 2, 3]` |
| `Dictionary` | 字典 | `let scores = ["A": 90]` |

### 字符串插值和运算

```swift
let name = "World"
let message = "Hello, \(name)!"

// 字符串操作
var str = "Swift"
str.append("!")
str += " Programming"

// 多行字符串保留格式
let poem = """
    Swift as the wind,
    Strong as a mountain,
    Elegant as poetry.
    """
```

## 可选类型（Optionals）

### 什么是可选类型

可选类型表示一个值可能存在，也可能不存在（nil）：

```swift
// 可选整数
var optionalNumber: Int? = 42
optionalNumber = nil

// 强制解包（危险！如果为 nil 会崩溃）
let forced = optionalNumber!

// 安全解包方式
if let number = optionalNumber {
    print("The number is \(number)")
} else {
    print("No number")
}

// Nil 合并运算符
let defaultNumber = optionalNumber ?? 0

// 可选链
let uppercased = optionalNumber?.description.uppercased()
```

### guard 语句

`guard` 语句用于提前退出，提高代码可读性：

```swift
func processUser(age: Int?, name: String?) {
    guard let validAge = age, validAge >= 0 else {
        print("Invalid age")
        return
    }

    guard let validName = name, !validName.isEmpty else {
        print("Invalid name")
        return
    }

    print("User: \(validName), Age: \(validAge)")
}

// 使用
processUser(age: 25, name: "Alice")  // 成功
processUser(age: -5, name: "Bob")    // Invalid age
processUser(age: 30, name: nil)      // Invalid name
```

### if let 和 guard let 对比

| 特性 | `if let` | `guard let` |
|------|----------|-------------|
| 作用域 | 仅在 if 块内 | 在 guard 之后的代码 |
| 使用场景 | 条件分支处理 | 前置条件检查 |
| 代码风格 | 嵌套可能较深 | 减少嵌套，提前退出 |

## 集合类型

### 数组和字典

```swift
// 数组
var fruits = ["Apple", "Banana", "Orange"]
fruits.append("Mango")
fruits.insert("Grape", at: 1)
let firstFruit = fruits[0]

// 字典
var scores: [String: Int] = [
    "Alice": 95,
    "Bob": 87,
    "Charlie": 92
]

scores["David"] = 88
if let aliceScore = scores["Alice"] {
    print("Alice scored \(aliceScore)")
}

// 遍历字典
for (name, score) in scores {
    print("\(name): \(score)")
}
```

### Set 集合

```swift
let favoriteGenres: Set<String> = ["Rock", "Classical", "Hip hop"]
let otherGenres: Set<String> = ["Jazz", "Rock", "Electronic"]

// 集合运算
let intersection = favoriteGenres.intersection(otherGenres)
let union = favoriteGenres.union(otherGenres)
let difference = favoriteGenres.symmetricDifference(otherGenres)

print("Both like: \(intersection)")
print("All genres: \(union)")
```

## 控制流

### 高级 switch

```swift
let character: Character = "a"

switch character {
case "a", "e", "i", "o", "u":
    print("\(character) is a vowel")
case "b", "c", "d", "f", "g", "h", "j", "k", "l", "m",
     "n", "p", "q", "r", "s", "t", "v", "w", "x", "y", "z":
    print("\(character) is a consonant")
default:
    print("\(character) is not a letter")
}

// 区间匹配
let count = 62
switch count {
case 0:
    print("none")
case 1..<5:
    print("a few")
case 5..<12:
    print("several")
case 12..<100:
    print("dozens")
case 100..<1000:
    print("hundreds")
default:
    print("many")
}

// 元组匹配
let point = (1, 1)
switch point {
case (0, 0):
    print("origin")
case (_, 0):
    print("on x-axis")
case (0, _):
    print("on y-axis")
case (-2...2, -2...2):
    print("inside the box")
default:
    print("outside")
}
```

### for-in 循环

```swift
// 遍历范围
for index in 1...5 {
    print("\(index) times 5 is \(index * 5)")
}

// 遍历数组
let names = ["Anna", "Alex", "Brian", "Jack"]
for name in names {
    print("Hello, \(name)!")
}

// 遍历字典
let numberOfLegs = ["spider": 8, "ant": 6, "cat": 4]
for (animalName, legCount) in numberOfLegs {
    print("\(animalName)s have \(legCount) legs")
}

// 带索引的遍历
for (index, name) in names.enumerated() {
    print("\(index + 1). \(name)")
}

// stride 函数
let minutes = 60
let minuteInterval = 5
for tickMark in stride(from: 0, to: minutes, by: minuteInterval) {
    print("Tick mark at \(tickMark) minutes")
}
```

## 函数和闭包

### 函数定义

```swift
// 基本函数
func greet(person: String) -> String {
    return "Hello, \(person)!"
}

// 参数标签和参数名
func greet(person: String, from hometown: String) -> String {
    return "Hello \(person)! Glad you could visit from \(hometown)."
}

let greeting = greet(person: "Bill", from: "Cupertino")

// 默认参数
func greet(person: String, nicely: Bool = true) -> String {
    if nicely {
        return "Hello, \(person)!"
    } else {
        return "Oh no, it's \(person) again..."
    }
}

print(greet(person: "Tim"))           // Hello, Tim!
print(greet(person: "Tim", nicely: false))  // Oh no, it's Tim again...

// 可变参数
func arithmeticMean(_ numbers: Double...) -> Double {
    var total: Double = 0
    for number in numbers {
        total += number
    }
    return total / Double(numbers.count)
}

print(arithmeticMean(1, 2, 3, 4, 5))
```

### 闭包（Closures）

```swift
// 基本闭包
let names = ["Chris", "Alex", "Ewa", "Barry", "Daniella"]

// 完整写法
let reversedNames = names.sorted(by: { (s1: String, s2: String) -> Bool in
    return s1 > s2
})

// 类型推断简化
let reversedNames2 = names.sorted(by: { s1, s2 in return s1 > s2 })

// 隐式返回
let reversedNames3 = names.sorted(by: { s1, s2 in s1 > s2 })

// 简写参数名
let reversedNames4 = names.sorted(by: { \$0 > \$1 })

// 尾随闭包
let reversedNames5 = names.sorted { \$0 > \$1 }

// 捕获值
func makeIncrementer(forIncrement amount: Int) -> () -> Int {
    var runningTotal = 0
    func incrementer() -> Int {
        runningTotal += amount
        return runningTotal
    }
    return incrementer
}

let incrementByTen = makeIncrementer(forIncrement: 10)
print(incrementByTen()) // 10
print(incrementByTen()) // 20
```

## 结构体与类

### 值类型 vs 引用类型

```swift
// 结构体 - 值类型
struct Resolution {
    var width = 0
    var height = 0
}

// 类 - 引用类型
class VideoMode {
    var resolution = Resolution()
    var interlaced = false
    var frameRate = 0.0
    var name: String?
}

let hd = Resolution(width: 1920, height: 1080)
var cinema = hd // 值拷贝
cinema.width = 2048

print("hd width: \(hd.width)")         // 1920
print("cinema width: \(cinema.width)")  // 2048

let tenEighty = VideoMode()
tenEighty.resolution = hd
tenEighty.interlaced = true
tenEighty.name = "1080i"
tenEighty.frameRate = 25.0

let alsoTenEighty = tenEighty // 引用拷贝
alsoTenEighty.frameRate = 30.0

print("tenEighty frameRate: \(tenEighty.frameRate)")  // 30.0
```

### 属性观察器

```swift
class StepCounter {
    var totalSteps: Int = 0 {
        willSet(newTotalSteps) {
            print("About to set totalSteps to \(newTotalSteps)")
        }
        didSet {
            if totalSteps > oldValue  {
                print("Added \(totalSteps - oldValue) steps")
            }
        }
    }
}

let stepCounter = StepCounter()
stepCounter.totalSteps = 200
// About to set totalSteps to 200
// Added 200 steps
stepCounter.totalSteps = 360
// About to set totalSteps to 360
// Added 160 steps
```

## 协议与扩展

### 协议定义

```swift
protocol FullyNamed {
    var fullName: String { get }
}

protocol RandomNumberGenerator {
    func random() -> Double
}

// 协议继承
protocol NamedAndAged: FullyNamed {
    var age: Int { get }
}

// 遵循协议
struct Person: FullyNamed {
    var firstName: String
    var lastName: String

    var fullName: String {
        return "\(firstName) \(lastName)"
    }
}

let john = Person(firstName: "John", lastName: "Appleseed")
print(john.fullName)
```

### 扩展

```swift
// 扩展现有类型
extension Int {
    func repetitions(task: () -> Void) {
        for _ in 0..<self {
            task()
        }
    }

    var squared: Int {
        return self * self
    }
}

3.repetitions {
    print("Hello!")
}

print(5.squared) // 25

// 协议扩展提供默认实现
extension RandomNumberGenerator {
    func randomBool() -> Bool {
        return random() > 0.5
    }
}
```

## 泛型

### 泛型函数和类型

```swift
// 泛型函数
func swapTwoValues<T>(_ a: inout T, _ b: inout T) {
    let temporaryA = a
    a = b
    b = temporaryA
}

var someInt = 3
var anotherInt = 107
swapTwoValues(&someInt, &anotherInt)

var someString = "hello"
var anotherString = "world"
swapTwoValues(&someString, &anotherString)

// 泛型类型
struct Stack<Element> {
    private var items: [Element] = []

    mutating func push(_ item: Element) {
        items.append(item)
    }

    mutating func pop() -> Element {
        return items.removeLast()
    }

    var topItem: Element? {
        return items.isEmpty ? nil : items[items.count - 1]
    }

    var isEmpty: Bool {
        return items.isEmpty
    }
}

var stackOfStrings = Stack<String>()
stackOfStrings.push("uno")
stackOfStrings.push("dos")
stackOfStrings.push("tres")
print(stackOfStrings.pop()) // tres

// 泛型约束
func findIndex<T: Equatable>(of valueToFind: T, in array: [T]) -> Int? {
    for (index, value) in array.enumerated() {
        if value == valueToFind {
            return index
        }
    }
    return nil
}
```

## 错误处理

### 定义和抛出错误

```swift
enum PrinterError: Error {
    case outOfPaper
    case noToner
    case onFire
}

func send(job: Int, toPrinter printerName: String) throws -> String {
    if printerName == "Never Has Toner" {
        throw PrinterError.noToner
    }
    return "Job sent"
}

// 使用 do-catch
func processPrintJob() {
    do {
        let printerResponse = try send(job: 1040, toPrinter: "Bi Sheng")
        print(printerResponse)
    } catch PrinterError.onFire {
        print("I'll just put this over here, with the rest of the fire.")
    } catch let printerError as PrinterError {
        print("Printer error: \(printerError).")
    } catch {
        print(error)
    }
}

// try? 转换为可选
let printerSuccess = try? send(job: 1884, toPrinter: "Mergenthaler")
let printerFailure = try? send(job: 1885, toPrinter: "Never Has Toner")

// defer
func processFile(filename: String) throws {
    let file = open(filename)
    defer {
        close(file)
    }
    // 处理文件...
    // 无论是否抛出错误，defer 都会执行
}
```

## 属性包装器

```swift
@propertyWrapper
struct TwelveOrLess {
    private var number = 0
    var wrappedValue: Int {
        get { return number }
        set { number = min(newValue, 12) }
    }
}

struct SmallRectangle {
    @TwelveOrLess var height: Int
    @TwelveOrLess var width: Int
}

var rectangle = SmallRectangle()
print(rectangle.height) // 0

rectangle.height = 10
print(rectangle.height) // 10

rectangle.height = 24
print(rectangle.height) // 12
```

## 总结

Swift 是一门现代化的编程语言，提供了丰富的特性：

1. **类型安全**：可选类型消除空指针异常
2. **值类型优先**：结构体和枚举都是值类型，减少副作用
3. **协议导向**：通过协议和扩展实现多态
4. **函数式特性**：闭包、高阶函数、不可变性
5. **现代语法**：类型推断、字符串插值、模式匹配

> Swift 的设计哲学是安全、快速、表达力强。它不仅仅是一门 iOS 开发语言，也是一门通用的现代编程语言。

通过本文的学习，你应该已经掌握了 Swift 的核心语法特性，可以开始构建 iOS、macOS 或其他平台的应用程序了。
$doc$,
    NULL,
    'published',
    NOW(),
    NOW(),
    NOW()
)
ON CONFLICT (slug) DO NOTHING;

-- 重置序列
SELECT setval('posts_id_seq', COALESCE((SELECT MAX(id) FROM posts), 1), false);
