# auto_tuple 设计文档

> 本文件是实现的唯一依据。核心语义、双轨道形态、重写规则、边界与错误处理均在此定稿。
> 状态：语义定稿，待实现。

## 1. 核心语义

对被选中 trait 项做**逐元素元组化**：

```rust
(x, y).foo(a, b, c) == (x.foo(a, b, c), y.foo(a, b, c))
(A, B)::foo(a, b, c) == (A::foo(a, b, c), B::foo(a, b, c))
(A, B)::MAX == (A::MAX, B::MAX)
(A, B)::Output == (A::Output, B::Output)
```

不存在算术合并等其他语义。N 元组版本即"每个元素调用原方法/取原常量/取原关联类型，结果拼成 N 元组"。

## 2. 辅助 trait：双轨道形态

`#[auto_tuple]` 处理 trait 定义，生成辅助 trait + blanket impl。**每个原 trait 只生成一个辅助 trait**（按轨道判定择一），不重复。

### 2.1 轨道判定

- 扫描**被选中项**的签名（方法参数/返回/泛型 bound/方法 where、关联常量类型、关联类型 bound）以及**原 trait 级 where**。
- 标识符解析：签名中出现的标识符，排除方法自身的泛型参数名后，命中 trait 泛型参数集 → 引用。类型路径（`visit_path`）与 lifetime（`visit_lifetime`，`&'a str` 场景）都要扫描。方法泛型参数与 trait 泛型参数**同名被 Rust 禁止**（E0403），排除逻辑仅为防御。
- 任一 trait 泛型参数被引用 → **共享轨道**；否则 → **异参轨道（All）**。
- All 轨道支持**任意参数形状**：每元素生成完整参数组（多 type 参数、lifetime、const 参数各自独立），元素可异参实例化。
- **带 bound 的 lifetime 参数**（`'b: 'a` / `'b: 'static`）→ 强制共享轨道（All 轨道无法逐元素表达 lifetime bound，保守正确）。
- 未选中的项不参与判定。

### 2.2 共享轨道（签名引用原泛型参数）

```rust
#[auto_tuple]
trait Tr<T> {
    fn g(x: &T) -> T;
    fn foo() -> Box<Self>;
    type Output;
}

trait _TrTuple2<A, B, T>
where
    A: Tr<T>,
    B: Tr<T>,
{
    fn g(x: &T) -> (T, T);
    fn foo() -> (Box<A>, Box<B>);
    type Output;                     // 仅用户显式选中时生成
}

impl<T, A: Tr<T>, B: Tr<T>> _TrTuple2<A, B, T> for (A, B) {
    fn g(x: &T) -> (T, T) { (A::g(x), B::g(x)) }
    fn foo() -> (Box<A>, Box<B>) { (A::foo(), B::foo()) }
    type Output = (A::Output, B::Output);
}
```

- 辅助 trait 泛型参数 = **元素参数（N 个，恒有，N≥1）+ 原 trait 全部泛型参数（按原序）**。
- 元素 bound `A_i: Tr<原参数...>` 恒写（0 元组除外）。
- A、B 共享同一参数化（语义收窄：异参不可行，见 §2.3 反向说明）。

### 2.3 异参轨道 / All（签名不引用原泛型参数）

```rust
#[auto_tuple]
trait Tr<T> {
    fn foo() -> Box<Self>;
    fn h() -> Self::Output;
    type Output;
}

trait _TrTuple2All<A, B, TA, TB>
where
    A: Tr<TA>,
    B: Tr<TB>,
{
    fn foo() -> (Box<A>, Box<B>);
    fn h() -> (A::Output, B::Output);
    type Output;
}

impl<TA, A: Tr<TA>, TB, B: Tr<TB>> _TrTuple2All<A, B, TA, TB> for (A, B) {
    fn foo() -> (Box<A>, Box<B>) { (A::foo(), B::foo()) }
    fn h() -> (A::Output, B::Output) { (A::h(), B::h()) }
    type Output = (A::Output, B::Output);
}
```

- 辅助 trait 泛型参数 = **元素参数 + 每元素完整参数组**（按元素序、原参数序）：type 参数 `__T{i}_{name}`（原 bounds 转移）、lifetime `'__L{i}_{name}`、const `const __C{i}_{name}: Ty`。
- 元素 bound `A: Tr<每元素参数组>` 允许**异参**（`A: Tr<i32, S>, B: Tr<String, U>`、`A: Tr<'a1>`、`A: Tr<3>` 各自独立）。
- 原 trait 参数若有 bound（`Tr<T: Clone>`），**转移到对应每元素参数**（`__T0_T: Clone`），保证 `A: Tr<TA>` 的 well-formedness。
- 每元素参数不出现在方法签名中，方法调用时由 `A0: Tr<?>` 约束反推；唯一则行、多则歧义（与直接调用 `A0::foo()` 行为一致）。歧义时可用 UFCS 显式指定消歧：

  ```rust
  <(A0, B0) as _TrTuple2All<A0, B0, i32, String>>::foo()
  ```

  （这正是保留 TA/TB 的理由：简化版无逃生舱。）

### 2.4 轨道差异汇总

| | 共享 | All |
|---|---|---|
| 触发 | 选中项签名引用原泛型参数 | 不引用 |
| 泛型参数 | 元素 + 原参数（共享） | 元素 + 各自 Tr 参数（异参） |
| 元素 bound | `A: Tr<T>` | `A: Tr<TA>` |
| 调用推断 | T 由签名提供或反推 | TA/TB 反推 |
| 歧义逃生 | UFCS 指定 T | UFCS 指定 TA/TB |

## 3. 类型重写规则

两条轨道共用同一套重写规则，仅辅助 trait 头的参数集不同。

### 3.1 返回方向（逐元素）

`Ret` → `(Elem(Ret, 0), ..., Elem(Ret, N-1))`，`Elem(T, i)` 递归定义：

```
Elem(Self, i)        = A_i            // 元素参数
Elem(Self::Assoc, i) = A_i::Assoc     // 元素投影
Elem(X, i)           = X              // X 为原泛型参数/方法泛型参数/具体类型：原样
Elem(&T, i)          = &Elem(T, i)    // 引用/容器/元组递归
Elem(Box<T>, i)      = Box<Elem(T, i)>
Elem(Vec<T>, i)      = Vec<Elem(T, i)>
Elem((T1, T2), i)    = (Elem(T1, i), Elem(T2, i))
```

- `fn f(&self) -> &Self` → `(&A, &B)`；`-> Box<Self>` → `(Box<A>, Box<B>)`；`-> Self` → `(A, B)`。
- lifetime 原样保留（含 elision；`(&A, &B)` 的 elided lifetime 绑定 `&self`，合法）。
- `fn foo();`（无返回类型）→ `-> ((), ())`。

### 3.2 参数方向（元组整体 + 拆包）

- 非 Self 相关参数：**原样**（`x: &T`、`x: usize` 等），转发时原样传给每个元素。
- Self 相关参数（TSR，一层）：辅助 trait 中保持原样（`Self` 即元组整体、`Self::Assoc` 展开为元素投影元组），impl 转发时**结构性拆包**：

  ```rust
  fn f(&self, x: &Self)     { (self.0.f(&x.0), self.1.f(&x.1)) }
  fn g(&mut self, x: &mut Self) { (self.0.g(&mut x.0), self.1.g(&mut x.1)) }
  fn h(x: Self)             { (self.0.h(x.0), self.1.h(x.1)) }   // 不同字段分别移动，合法
  fn k(x: Self::Output)     { (self.0.k(x.0), self.1.k(x.1)) }
  ```

- TSR 集合：`Self`、`Self::Assoc`、`&Self`、`&mut Self`、`&Self::Assoc`、`&mut Self::Assoc`。
- **嵌套容器中的 Self 作参数**（`Box<Self>`、`Vec<Self>`、`(Self, T)`）→ 无法拆包，compile_error。

### 3.3 机械重写铁律

> 只做结构变换（Self → 元素参数、包元组、拆包），**绝不改写任何泛型参数标识符**。

方法泛型参数遮蔽 trait 泛型参数（`fn g<T>(x: T)`）时，标识符落到哪层由 Rust 解析器按遮蔽规则决定，机械重写天然保留正确语义。元素参数（`__T{i}`）、`Tr` 参数（`__TA{i}`）与合成参数（`__arg{i}`）**自动避让**原 trait 泛型参数名与已有参数名（冲突时追加 `_`），不再假设前缀足够独特。

## 4. 各项生成规则

### 4.1 方法

- 任意形态：`&self` / `&mut self` / `self` / 关联函数 / 泛型方法 / 带 where / 带 lifetime。
- 转发 body 按 §3 规则生成。
- **按值参数**（非 TSR）：直接原样生成 `(A::g(x), B::g(x))`，**不做 Copy/Clone 预判**——是 Copy 则编译通过，否则 E0382 由编译器报告（宏无法静态判断具体类型是否 Copy，也不自动加 bound 改变语义）。
- `&mut T` 参数：reborrow 转发 `(self.0.g(&mut *x), self.1.g(&mut *x))`。
- 默认方法（带 body）：辅助 trait 中**无 body**，body 由生成的 impl 给出；原默认实现经 `self.0.foo()` 自然继承。
- `async fn`：顺序 await，`(self.0.foo(x).await, self.1.foo(x).await)`。
- `impl Trait`（RPIT）返回：逐元素展开为每个元素一个 opaque，`-> impl Iterator<Item = Self>` → `(impl Iterator<Item = A>, impl Iterator<Item = B>)`；impl 的隐藏类型是元素各自 `Tr::foo` 的 opaque（已验证可行）。
- 多层关联投影（`Self::Output::Item`）在返回类型与 where 主语中逐元素化为 `A::Output::Item`——Rust 对泛型参数上的嵌套投影有固有限制（E0223，与裸写 `T::Out::Item` 一致）；消除需 UFCS 形式（`<<A as Tr>::Output as Iterator>::Item`），列为后续增强。
- `unsafe fn`：辅助 trait 保留 `unsafe`，生成的 impl body 显式 `unsafe {}` 块（edition 2024 无隐式 unsafe body）。
- `unsafe trait`：辅助 trait 标 `unsafe`，impl 为 `unsafe impl`。

### 4.2 关联常量

- 类型按 §3.1 元组化（`usize` → `(usize, usize)`；`Self` → `(A, B)`；`T` → `(T, T)`）。
- 值逐元素：`const MAX: (usize, usize) = (A::MAX, B::MAX);`（const 上下文合法）。
- **默认不处理**，仅用户显式指定时生成。

### 4.3 关联类型

- 辅助 trait 声明 `type Output;`（无默认），impl 给定 `type Output = (A::Output, B::Output);`。
- 方法签名中的 `Self::Output` 展开为元素投影 `(A::Output, B::Output)`（bound 已满足，无需辅助 trait 自带关联类型）。
- **原 bound 处理（白名单）**：辅助 trait 的关联类型 bound 只保留**元组必然满足**的（`Clone`/`Copy`/`PartialEq`/`Eq`/`PartialOrd`/`Ord`/`Hash`/`Debug`/`Default`/`Sized` 无参形式、lifetime）。其余一律删除（`Iterator`、`Add`、`AsRef<T>`、含 `Self`/原参数的 bound）——元组值无法满足，元素侧已由 `A: Tr<...>` 保证。
- **默认不处理**，仅用户显式指定时生成。

### 4.4 方法级 / trait 级 where

- `where Self: Foo` → 拆分为 `where A: Foo, B: Foo`（1 元组单个，0 元组删除）。
- `where Self::Output: Clone` → 元素投影 `A::Output: Clone, B::Output: Clone`。
- `where Self: Sized` → 元素化后恒真，无害。
- 复杂嵌套（`where Vec<Self>: Foo`）→ compile_error。
- 原 trait 级 where 原样复制进辅助 trait 与 impl。

### 4.5 impl where 完整性

impl 泛型参数 = 辅助 trait 声明的**全部 bound 原样复制**（原参数 bound、元素 bound、元素化关联类型 bound）+ 原 trait where。

## 5. 元数与范围

- 默认 `2..=12`；支持 `2..12`、`0..=12`、`1..=3` 等（严格按 Rust 范围语义）。
- 0 元组：`_TrTuple0<T>` / `_TrTuple0All`（无参数）。无元素 bound。方法返回 `()`、body 空（不调用任何元素）；常量 `= ()`；关联类型 `= ()`。
- 1 元组：`_TrTuple1<A, T> where A: Tr<T>` / `_TrTuple1All<A, TA> where A: Tr<TA>`。语法注意 `(A,)`、`(T,)`、`(expr,)` 补逗号。
- 1 元组按值参数只调用一次，无移动问题；0 元组不调用。

## 6. 遮蔽与标识符

- 重写器不改标识符（§3.3）。
- 方法泛型参数与 trait 泛型参数**同名被 Rust 禁止**（E0403）——遮蔽场景不存在，轨道判定即简单的名字匹配（analyze 中排除方法参数名仅为防御）。
- 元素参数独特前缀名（`__T0`、`__T1`…），共享轨道元素 bound 用 where 子句（`trait _TrTuple2<A, B, T = i32> where A: Tr<T>, B: Tr<T>`，默认值参数放最后）。

## 7. 命名与可见性

- 共享：`_{Trait}Tuple{N}`；异参：`_{Trait}Tuple{N}All`（前导下划线）。
- 生成在原 trait 所在模块，可见性与原 trait 相同（pub trait → pub 辅助 trait），`#[doc(hidden)]`。
- 可命名覆盖（同时覆盖两个轨道名）。
- 跨模块同名 trait 不冲突（模块隔离）；用户手动实现同名辅助 trait → E0119，文档约定 + 可命名规避。
- 生成名（元素参数 `__T{i}`、每元素参数组 `__T{i}_{name}`/`'__L{i}_{name}`/`__C{i}_{name}`、合成参数 `__arg{i}`）自动避让原 trait 参数名与已有参数名。

## 8. 错误处理（compile_error）

- 非法范围 / 非法筛选语法 / 指定的项不存在 / 多个 range。
- 参数方向嵌套容器 Self。
- 按值 `impl Trait` 参数（无法转发给多个元素）。
- 参数中 `impl Trait` 含 Self（`fn f(x: impl Iterator<Item = Self>)`——单值无法同时满足各元素的 Item 约束）。
- 自定义 receiver（`self: Box<Self>` 等）。
- 返回中 `+ use<..>` precise capturing（trait/impl 两侧的 per-element opaque 捕获列表尚未处理）。
- where 中复杂 Self 嵌套。
- auto trait。
- 空选择集提示：`#[auto_tuple()]` 与 `#[auto_tuple]` 均视为默认配置（处理全部方法），无显式空选择集写法。

## 9. 不支持边界（交给编译器）

- 按值参数非 Copy（E0382，编译器报告）。
- 泛型 async fn（若工具链不支持，编译器报告）。
- 重复 `#[auto_tuple]`（E0428 重复定义，编译器报告）。

## 10. 测试策略

- trybuild 编译测试矩阵：简单/泛型 trait、`&self`/`&mut self`/`self`、关联函数、泛型方法、async、RPIT、关联常量/类型、筛选、双轨道、0/1 元组、默认值、命名冲突。
- 运行期断言验证逐元素语义（`(A::foo(), B::foo())` 等价性）。
- 属性解析单元测试（范围、筛选、非法输入）。

## 11. 待验证项

1. ~~方法调用 `(a, b).foo()` 与关联函数 `(A, B)::foo()` 的泛型参数推断~~ — 已验证通过（共享轨道 T 由签名提供 / All 轨道 TA/TB 反推，trybuild 实测）。
2. ~~`&Self` 返回 elision 元组化~~ — 已验证（`fn name(&self) -> &str` → `(&str, &str)`）。
3. ~~edition 2024 unsafe body 处理~~ — 已验证（unsafe fn body 显式 `unsafe {}`）。
4. ~~泛型 async fn 在目标工具链的行为~~ — 已验证：原生 `async fn`（含参数）逐元素顺序 await 正常；泛型 async 方法由编译器裁决。
5. ~~RPIT 返回（`impl Trait`）~~ — 已验证支持：逐元素展开为每元素一个 opaque，隐藏类型为元素各自的 opaque。
