import org.graalvm.buildtools.gradle.tasks.BuildNativeImageTask

plugins {
    application
    java
    id("org.graalvm.buildtools.native") version "0.11.1"
}

group = "com.example"
version = "1.0.0"
val libVersion = providers
    .gradleProperty("libVersion")
    .orElse(System.getenv("LIB_VERSION") ?: "0.0.0-dev")

val ciMavenRepo = providers
    .gradleProperty("ciMavenRepo")
    .orElse(System.getenv("CI_MAVEN_REPO") ?: "")

repositories {
    mavenCentral()
    if (ciMavenRepo.get().isNotBlank()) {
        maven {
            name = "ci"
            url = uri(ciMavenRepo.get())
        }
    }
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(21))
    }
}

application {
    mainClass.set("io.getunleash.example.BuildValidator")
}

dependencies {
    implementation("io.getunleash:yggdrasil-engine:${libVersion.get()}")
}

tasks.test {
    useJUnitPlatform()
}

tasks.register("printHostInfo") {
    doLast {
        println("os.name      = ${System.getProperty("os.name")}")
        println("os.arch      = ${System.getProperty("os.arch")}")
        println("java.version = ${System.getProperty("java.version")}")
        println("java.vendor  = ${System.getProperty("java.vendor")}")
    }
}

/**
 * Task 1: plain JVM probe
 *
 * Runs a tiny Java app that just constructs UnleashEngine.
 * If it exits 0, loading worked.
 */
tasks.register<JavaExec>("runJvmProbe") {
    group = "verification"
    description = "Runs the JVM probe app that instantiates UnleashEngine"

    classpath = sourceSets["main"].runtimeClasspath
    mainClass.set(application.mainClass)

    // Optional: pass through library path if you need it
    if (System.getenv("JAVA_LIBRARY_PATH") != null) {
        jvmArgs("-Djava.library.path=${System.getenv("JAVA_LIBRARY_PATH")}")
    }
}

val nativeVariant = providers.gradleProperty("nativeVariant").orElse("graal")

graalvmNative {
    binaries {
        named("main") {
            imageName.set(
                when (nativeVariant.get()) {
                    "graal" -> "unleash-engine-probe-graal"
                    "mostlyStatic" -> "unleash-engine-probe-mostly-static"
                    "staticMusl" -> "unleash-engine-probe-static-musl"
                    else -> error("Unknown nativeVariant=${nativeVariant.get()}")
                }
            )

            fallback.set(false)
            verbose.set(true)

            buildArgs.add("-O3")
            buildArgs.add("--enable-url-protocols=http,https")

            when (nativeVariant.get()) {
                "graal" -> {
                    // regular native image
                }
                "mostlyStatic" -> {
                    buildArgs.add("--static-nolibc")
                }
                "staticMusl" -> {
                    buildArgs.add("--static")
                    buildArgs.add("--libc=musl")
                }
            }
        }
    }
}

/**
 * This is the real task created by the plugin.
 * Safe to customize because it already exists and is wired.
 */
tasks.named<BuildNativeImageTask>("nativeCompile") {
    // optional extra tuning here
}

/**
 * Convenience aliases so GitHub Actions can call separate task names.
 * These use nested Gradle invocations, which Gradle supports via GradleBuild.
 */
tasks.register<GradleBuild>("buildGraalProbe") {
    group = "build"
    description = "Builds a regular Graal native executable"
    tasks = listOf("nativeCompile")
    startParameter.projectProperties = mapOf(
        "nativeVariant" to "graal",
        "libVersion" to libVersion.get()
    )
}

tasks.register<GradleBuild>("buildGraalMostlyStatic") {
    group = "build"
    description = "Builds a mostly-static Graal native executable"
    tasks = listOf("nativeCompile")
    startParameter.projectProperties = mapOf(
        "nativeVariant" to "mostlyStatic",
        "libVersion" to libVersion.get()
    )
}

tasks.register<GradleBuild>("buildGraalStaticMusl") {
    group = "build"
    description = "Builds a fully static musl Graal native executable"
    tasks = listOf("nativeCompile")
    startParameter.projectProperties = mapOf(
        "nativeVariant" to "staticMusl",
        "libVersion" to libVersion.get()
    )
}