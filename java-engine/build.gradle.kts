plugins {
    `java-library`
    `maven-publish`
    signing
    id ("com.diffplug.spotless").version("8.0.0")
    id("io.github.gradle-nexus.publish-plugin").version("2.0.0")
    id("pl.allegro.tech.build.axion-release").version("1.21.0")
    id("tech.yanand.maven-central-publish").version("1.3.0")
    id("me.champeau.jmh").version("0.7.3")
}

version = project.findProperty("version") as String

val binariesDir = file("binaries")
val sonatypeUsername: String? by project
val sonatypePassword: String? by project
val signingKey: String? by project
val signingPassphrase: String? by project
val mavenCentralToken: String? by project

repositories {
    mavenCentral()
}

dependencies {
    jmh(libs.jmh.core)
    jmhAnnotationProcessor(libs.jmh.annprocess)
    testImplementation(platform(libs.junit.bom))
    testImplementation("org.junit.jupiter:junit-jupiter-api")
    testImplementation("org.junit.platform:junit-platform-launcher")
    testImplementation("org.junit.jupiter:junit-jupiter-engine")
    testImplementation("org.junit.jupiter:junit-jupiter-params")
    testImplementation(libs.mockito.core)
    testImplementation(libs.slf4j.simple)
    implementation(libs.slf4j.api)
    implementation(libs.jna)
    implementation(libs.jackson.core)
    implementation(libs.jackson.databind)
    implementation(libs.jackson.jsr310)
    implementation(libs.flatbuffers)
}

tasks.jar {
    manifest {
        attributes(
            "Implementation-Title" to project.name,
            "Implementation-Version" to project.version
        )
    }
    from(binariesDir) {
        into("native")
    }
}

val buildFfi by tasks.registering(Exec::class) {
    group = "build"
    description = "Builds the Rust Ffi binary"

    workingDir = file("../yggdrasilffi")
    commandLine = listOf("cargo", "build", "--release")
}

val copyTestBinary by tasks.register<Copy>("copyTestBinary") {
    val platform = System.getProperty("os.arch").lowercase()
    val os = System.getProperty("os.name").lowercase()

    val sourceFileName = when {
        os.contains("linux") -> "libyggdrasilffi.so"
        os.contains("mac") -> "libyggdrasilffi.dylib"
        os.contains("win") -> "yggdrasilffi.dll"
        else -> throw GradleException("Unsupported OS")
    }

    val sourcePath = file("../target/release/$sourceFileName")
    val targetPath = file("build/resources/test/native")

    val binaryName = when {
        os.contains("mac") && platform.contains("arm") -> "libyggdrasilffi_arm64.dylib"
        os.contains("mac") -> "libyggdrasilffi_x86_64.dylib"
        os.contains("win") -> "yggdrasilffi_x86_64.dll"
        os.contains("linux") -> "libyggdrasilffi_x86_64.so"
        else -> throw GradleException("Unsupported OS")
    }

    from(sourcePath) {
        rename { binaryName }
    }
    into(targetPath)
    outputs.upToDateWhen { false }
}

copyTestBinary.dependsOn(buildFfi)

tasks.named<Test>("test") {
    dependsOn(copyTestBinary)
    useJUnitPlatform()
    testLogging { exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL }
}

tasks.named("jmh") {
    dependsOn(copyTestBinary)
}

tasks.named("jmhJar") {
    dependsOn(copyTestBinary)
}

spotless {
    java {
        googleJavaFormat("1.17.0")
        target("src/**/*.java")
    }
}

publishing {
    publications {
        create<MavenPublication>("mavenJava") {
            from(components["java"])

            pom {
                name.set("Unleash Yggdrasil Engine")
                description.set("Yggdrasil engine for computing feature toggles")
                url.set("https://docs.getunleash.io/yggdrasil-engine/index.html")
                licenses {
                    license {
                        name.set("MIT")
                        url.set("https://opensource.org/license/mit/")
                    }
                }
                developers {
                    developer {
                        id.set("chrkolst")
                        name.set("Christopher Kolstad")
                        email.set("chriswk@getunleash.io")
                    }
                    developer {
                        id.set("ivarconr")
                        name.set("Ivar Conradi Østhus")
                        email.set("ivarconr@getunleash.io")
                    }
                    developer {
                        id.set("gastonfournier")
                        name.set("Gaston Fournier")
                        email.set("gaston@getunleash.io")
                    }
                    developer {
                        id.set("sighphyre")
                        name.set("Simon Hornby")
                        email.set("simon@getunleash.io")
                    }
                }
                scm {
                    connection.set("scm:git:https://github.com/Unleash/yggdrasil")
                    developerConnection.set("scm:git:ssh://git@github.com:Unleash/yggdrasil")
                    url.set("https://github.com/Unleash/yggdrasil")
                }
            }
        }
    }
}

java {
    withSourcesJar()
    withJavadocJar()
    sourceCompatibility = JavaVersion.VERSION_11
    targetCompatibility = JavaVersion.VERSION_11
}

signing {
    if (signingKey != null && signingPassphrase != null) {
        useInMemoryPgpKeys(signingKey, signingPassphrase)
        sign(publishing.publications)
    }
}

mavenCentral {
    authToken = mavenCentralToken
    publishingType = "AUTOMATIC"
    maxWait = 120
}
