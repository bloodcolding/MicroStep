# rustls-platform-verifier Kotlin 组件经 JNI 按类名查找（ADR-008）。
# release 构建 isMinifyEnabled=true 时 R8 重命名/移除会导致 TLS 校验
# 运行时 ClassNotFoundException，必须 keep。
-keep class org.rustls.platformverifier.** { *; }
