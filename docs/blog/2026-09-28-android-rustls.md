# 当 rustls 遇上 Android：一场被 "An IO error occurred" 骗了三天的排障

> 一个 Tauri v2 + Rust 的跨平台应用，桌面端 Git 同步全绿，Android 端却始终报一个被层层包装的 IO 错误。TCP 能连上、curl 能通、权限没问题、PAT 换了也没用——最后发现凶手藏在依赖特性矩阵的一次静默变更里。

## 一、背景：一个"移动端就绪"的双栈 Git 同步引擎

我在做 [MicroStep](https://github.com/bloodcolding/MicroStep)——一个事件溯源的个人成长 RPG，Tauri v2 桌面/移动一体。用户数据落在系统 AppData 目录下的一个独立 git 仓库里，通过 GitHub 私有仓库在多设备间同步。

同步引擎是刻意的双栈设计：

- **fetch 走 gix**（纯 Rust 实现，无子进程——移动端沙盒红线）
- **push 走 git2-rs**（libgit2 进程内绑定）

```toml
gix = { version = "0.87.1", default-features = false,
        features = ["blocking-http-transport-reqwest-rust-tls", "sha1"] }
git2 = { version = "0.20", default-features = false,
         features = ["vendored-libgit2", "https"] }
```

桌面端双向同步早就全绿。打完 Android 包（rc2）装进模拟器，同步失败。

## 二、症状：一个教科书级的"烟雾弹"

Android 端的错误文案有两种，随机出现：

```
Didn't find 'application/x-git-upload-pack-advertisement' header to indicate
'smart' protocol, and 'dumb' protocol is not supported.

An IO error occurred when talking to the server
```

迷惑性拉满的证据链：

| 观察 | 结论倾向 |
|---|---|
| 模拟器 shell `curl` 直连 GitHub smart HTTP → **200 + advertisement** | 系统级网络没问题 |
| `tcpdump` 能看到 App 发起的到 `20.205.243.166:443`（GitHub）的 TCP 连接 | App 有网、能连上 |
| OhMyData 私有仓库未认证访问 → 401 | 服务端行为正常 |
| `android.permission.INTERNET` 已授予 | 不是权限 |
| 换新 PAT、修 URL 大小写、确认 APK 版本 | 全部无效 |

TCP 能连上、却拿不到 git 协议响应——像极了"认证没带上，服务端回了 404 页面"。我们顺着这条路修了 URL scheme 大小写（移动键盘自动大写 `Https://` 导致 PAT 注入失败）、PAT 清洗，发布了 rc2。

依然失败。

## 三、专家会诊：四个假设，代码级核验

我带着日志找专家看，得到四个经典方向：

1. **认证"裸奔"**，服务端 301 到登录页，gix 读到 HTML
2. **TLS/CA 在移动端找不到系统证书**，建议强制 rustls + webpki-roots
3. **User-Agent 缺失被 WAF 拦截**
4. **公共 Wi-Fi 强制门户**

专家还给了条金子般的建议：**把服务端返回的 payload 打出来，瞬间破案**。

逐条对照源码核验：

- 假设 1：我们的 `authenticated_url()` 早已是 `https://microstep:{pat}@` URL userinfo 注入形态——机制存在（而且 TLS 不通的话根本轮不到认证层）
- 假设 3：gix-transport 的 reqwest 后端默认带 git 风格 UA
- 假设 4：模拟器 curl 全通，排除

假设 2 方向正确，但**具体形态完全出乎意料**。

## 四、破案：读依赖链源码，挖出特性矩阵的静默变更

从我们声明的 feature 一路往下读：

**第一层**：`blocking-http-transport-reqwest-rust-tls` → gix-transport 启用 reqwest 的 `rustls` 特性。

**第二层**（关键）：reqwest **0.13 已经删掉了** `rustls-tls-webpki-roots` / `rustls-tls-native-roots` 这一整套特性。现在的 `rustls` 特性展开是：

```toml
rustls = [
  "__rustls-aws-lc-rs",
  "dep:rustls-platform-verifier",   # ← 唯一的证书验证器
  "__rustls",
]
```

也就是说：**想用 rustls？验证器只有 rustls-platform-verifier，没有第二条路。**

**第三层**：rustls-platform-verifier 0.7 在 Android 上的契约。翻开 `src/android.rs`：

```rust
fn global() -> &'static GlobalStorage {
    GLOBAL
        .get()
        .expect("Expect rustls-platform-verifier to be initialized")  // ← 凶器
}
```

文档写得很清楚：Android 上必须在任何校验发生前，通过 **JNI** 注入 JavaVM + Context 完成初始化，并且 App 的构建里要包含它配套的 Kotlin 组件（`org.rustls.platformverifier.CertificateVerifier`，随 `rustls-platform-verifier-android` crate 以 maven AAR 形式分发）。

**第四层**：搜遍 Tauri 2.11.6 源码——它自己根本不引用 rustls-platform-verifier（它那个 `rustls-tls` 特性是给 dev server 用的，ring provider）。**没有任何人会替你做这个初始化。**

于是完整的事故链：

```
App 发起 HTTPS
→ TCP 握手成功（所以 tcpdump 看得到连接 ✓）
→ 服务端证书到达，rustls 调 platform-verifier 校验
→ global() 未初始化 → panic!
→ panic 发生在 reqwest 内部线程，被 catch 住，App 不崩（所以 UI 正常 ✓）
→ 错误冒泡成 "An IO error occurred when talking to the server"
→ 而独立进程的 shell curl 自带证书逻辑，一切正常 ✓
```

每一个"矛盾"的观察都严丝合缝。

顺带还挖出一个相邻地雷：`openssl-probe` 的候选证书目录在 Android 上只有 Termux 私有路径，没有 `/system/etc/security/cacerts`——传统 rustls-native-certs 路线在 Android 同样不可行。

## 五、铁证：真机 logcat 当场抓获 panic 现场

修复版发布前，我先用 adb 连上 Redmi 真机，冷启动 rc2 抓日志：

```
$ adb logcat -s RustStdoutStderr

thread 'reqwest-internal-sync-runtime' panicked at
  rustls-platform-verifier-0.7.0/src/android.rs:90:10:
Expect rustls-platform-verifier to be initialized

thread '<unnamed>' panicked at reqwest-0.13.5/src/blocking/client.rs:1582:5:
event loop thread panicked
```

注意 tag `RustStdoutStderr`——tao 的 Android 胶水层会把 Rust 进程的 stdout/stderr 重定向进 logcat，`eprintln!` 直接可见。**排障移动端 Rust 应用，先把这个管道找通，等于有了 console。**

## 六、修复：四层外科手术

### 1. 入口：手写展开 `mobile_entry_point`，插一个 shim

Tauri 的 `mobile_entry_point` 宏把 setup 钩子硬编码给了 wry 的 `android_setup`。但 `tauri::android_binding!` 宏的 `$wry` 参数是个 module path——那就包一层：

```rust
#[cfg(target_os = "android")]
pub(crate) mod android_wry_glue {
    pub use tauri::wry::prelude;

    pub unsafe fn android_setup(package: &str, mut env: JNIEnv,
                                looper: &ThreadLooper, activity: GlobalRef) {
        // 先挂 TLS 初始化，再转调 wry 原始 setup
        init_platform_verifier(&mut env, &activity);
        tauri::wry::android_setup(package, env, looper, activity)
    }
}

#[cfg(target_os = "android")]
fn _start_app() {
    ::tauri::android_binding!(com_microstep, app, _start_app, android_wry_glue);
    stop_unwind(run);
}
```

`android_setup(package, env, looper, activity)` 正好握着 JNIEnv 和 Activity（Activity 即 Context）——现成的初始化时机。

### 2. JNI 0.21 ↔ 0.22 裸指针桥接

坑中坑：tao/wry 栈用 jni 0.21，rustls-platform-verifier 用 jni 0.22。两个大版本包装的是同一套 JNI C ABI，按裸指针互通：

```rust
let raw_env = env.get_raw() as *mut jni::sys::JNIEnv;
let raw_activity = activity.as_obj().as_raw() as jni::sys::jobject;
let mut unowned = unsafe { jni::EnvUnowned::from_raw(raw_env) };
unowned.with_env(|env| {
    let context = unsafe { jni::objects::JObject::from_raw(env, raw_activity) };
    rustls_platform_verifier::android::init_with_env(env, context)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
})
```

### 3. Gradle：接入 crate 自带的 maven AAR

Kotlin 组件不上 Maven Central，但就躺在 crate 包里。按官方 README 方式，用 `cargo metadata` 让 Gradle 自己定位：

```kotlin
fun rustlsPlatformVerifierMaven(): File {
    val metadataText = providers.exec {
        workingDir = File(project.rootDir, "../../")
        commandLine("cargo", "metadata", "--format-version", "1",
                    "--filter-platform", "aarch64-linux-android")
    }.standardOutput.asText.get()
    val metadata = groovy.json.JsonSlurper().parseText(metadataText) as Map<*, *>
    val manifestPath = (metadata["packages"] as List<*>)
        .first { (it as Map<*, *>)["name"] == "rustls-platform-verifier-android" }
        .let { (it as Map<*, *>)["manifest_path"] as String }
    return File(manifestPath).parentFile.resolve("maven")
}

repositories { maven { url = uri(rustlsPlatformVerifierMaven()) } }
dependencies { implementation("rustls:rustls-platform-verifier:0.1.1") }
```

### 4. R8 keep：一个 debug 正常、release 崩溃的暗雷

release 构建 `isMinifyEnabled = true`，R8 会重命名类——而验证器是**JNI 按类名反射查找** Kotlin 类的。不加 keep，debug 包能用、release 包炸 `ClassNotFoundException`：

```
-keep class org.rustls.platformverifier.** { *; }
```

### 附赠：把错误链捞回来

这次排障最大的时间黑洞是错误被层层 `.to_string()` 吞掉。修复版统一走完整 error chain + PAT 脱敏（`microstep:****@`，日志和截图不泄密）：

```rust
fn error_chain(err: &dyn std::error::Error) -> String {
    let mut chain = err.to_string();
    let mut source = err.source();
    while let Some(err) = source {
        chain.push_str("；原因: ");
        chain.push_str(&err.to_string());
        source = err.source();
    }
    chain
}
```

依赖侧零新增：`jni 0.22` 和 `rustls-platform-verifier 0.7` 本来就是 gix 的传递依赖，只是声明为直依赖好调用——Cargo.lock 只多了两条边。

## 七、闭环

```
push master → CI（含 aarch64 android-check）✅
tag v0.2.1-rc3 → 五平台 release ✅（Gradle 集成同时得到验证）
adb install -r → 冷启动
→ AGENT-DEBUG: rustls-platform-verifier initialized ✅
→ panic 消失 ✅
→ 手动同步：fetch + merge + push 全链路成功 ✅
```

## 八、复盘：五条带得走的教训

1. **移动端"能编译 ≠ 能运行"**。Rust 生态的平台抽象层在 Android 上常有隐式运行时契约（JNI 初始化、系统服务桥接），编译器不会告诉你。
2. **依赖的特性矩阵会变**。reqwest 0.12 → 0.13 把 webpki/native-roots 特性整个删了，`rustls` 的语义从"多种根证书选项"变成"platform-verifier 单行道"。升级大版本时，changelog 里 feature 的 Breaking 比 API 的 Breaking 更隐蔽。
3. **被吞掉的 panic 比错误更可怕**。它在别人线程里爆炸、以无辜 IO 错误的面目冒泡。凡是有后台线程 + 网络库的架构，先想清楚 panic 去哪了。
4. **先建可观测性，再改配置**。`stdout → logcat`（tag `RustStdoutStderr`）和完整 error chain 是这次能破案的两块基石；没有它们，再多假设也只是猜。
5. **专家意见要核验，但别轻视方向**。四个假设里三个不成立，但"TLS/CA 移动端水土不服"的大方向直接把调查引向了正确楼层——只是真凶的房间号，要靠读源码自己找。

---

*MicroStep 是一个开源的事件溯源个人成长 RPG，本文涉及的完整修复见 [commit 7ec088c](https://github.com/bloodcolding/MicroStep/commit/7ec088c) 与 [ADR-008](https://github.com/bloodcolding/MicroStep/blob/master/docs/harness/DECISIONS.md)。*
