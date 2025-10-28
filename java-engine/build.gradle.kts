import java.net.URL
import java.nio.file.Files
import java.util.zip.ZipEntry
import java.util.zip.ZipInputStream

plugins {
    java
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
    testImplementation(libs.assert4j.core)
    testImplementation(libs.mockito.core)
    testImplementation(libs.slf4j.simple)
    testImplementation(libs.jackson.core)
    testImplementation(libs.jackson.databind)
    testImplementation(libs.jackson.jsr310)
    implementation(libs.slf4j.api)
    implementation(libs.flatbuffers)
}

tasks.withType<Javadoc> {
    exclude("io/getunleash/messaging/**")
    exclude("io/getunleash/engine/MetricsBucket.java")
    exclude("io/getunleash/engine/Payload.java")
    exclude("io/getunelash/engine/IStrategy.java")
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
        os.contains("mac") && (platform.contains("arm") || platform.contains("aarch")) -> "libyggdrasilffi_arm64.dylib"
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
    dependsOn(fetchClientSpecification)
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


@CacheableTask
abstract class FetchZip: DefaultTask() {
    @get:Input
    abstract val versionProp: Property<String>

    /**
     * Either provide a fully-resolved URL (e.g. via a `map` from version),
     * or set this directly from a property.
     */
    @get:Input
    abstract val downloadUrl: Property<String>

    @get:OutputDirectory
    abstract val destinationDir: DirectoryProperty

    init {
        // Up-to-date if destination/package.json exists AND its "version" equals versionProp
        outputs.upToDateWhen {
            val pkg = destinationDir.file("package.json").get().asFile
            if (!pkg.exists()) return@upToDateWhen false
            val text = pkg.readText()
            // Minimal/robust-enough extraction of "version": "x.y.z"
            val regex = """"version"\s*:\s*"([^"]+)"""".toRegex()
            val found = regex.find(text)?.groupValues?.getOrNull(1)
            found == versionProp.get()
        }
    }
    @TaskAction
    fun fetch() {
        val dest = destinationDir.get().asFile
        dest.mkdirs()

        // If an old version is there (and package.json doesn't match), we overwrite the folder.
        // We’ll unpack into a temp dir then atomically replace contents to avoid half-extracted states.
        val tmpZip = Files.createTempFile("download-", ".zip").toFile()
        val tmpExtractDir = Files.createTempDirectory("extract-").toFile()

        try {
            // --- Download ---
            logger.lifecycle("Downloading ${downloadUrl.get()} (version=${versionProp.get()})")
            URL(downloadUrl.get()).openStream().use { input ->
                tmpZip.outputStream().use { output -> input.copyTo(output) }
            }

            // Unzip while STRIPPING the first path segment (top-level folder) from all entries
            unzipStripTopLevel(tmpZip, tmpExtractDir)

            dest.listFiles()?.forEach { it.deleteRecursively() }
            tmpExtractDir.copyRecursively(dest, overwrite = true)

            logger.lifecycle("Unpacked to: $dest")
        } finally {
            tmpZip.delete()
            tmpExtractDir.deleteRecursively()
        }
    }

    /**
     * Unzips [zipFile] into [toDir], removing the first path segment from each entry.
     * Example: client-specification-1.2.3/pkg/a.json -> pkg/a.json
     * Also guards against Zip Slip attacks.
     */
    private fun unzipStripTopLevel(zipFile: java.io.File, toDir: java.io.File) {
        val targetCanonical = toDir.canonicalFile.toPath()

        fun stripFirstSegment(path: String): String {
            val parts = path.split('/','\\')
            return when {
                parts.isEmpty() -> ""
                parts.size == 1 -> parts[0] // single file at root (rare) — keep as is
                else -> parts.drop(1).joinToString("/")
            }
        }

        ZipInputStream(zipFile.inputStream()).use { zis ->
            var entry: ZipEntry? = zis.nextEntry
            while (entry != null) {
                val stripped = stripFirstSegment(entry.name).trim().removePrefix("/").removePrefix("\\")
                if (stripped.isNotEmpty()) {
                    val outPath = File(toDir, stripped)
                    // Zip Slip protection
                    val outCanonical = outPath.canonicalFile.toPath()
                    require(outCanonical.startsWith(targetCanonical)) {
                        "Blocked suspicious zip entry: ${entry.name}"
                    }

                    if (entry.isDirectory || stripped.endsWith("/") || stripped.endsWith("\\")) {
                        outPath.mkdirs()
                    } else {
                        outPath.parentFile?.mkdirs()
                        outPath.outputStream().use { os -> zis.copyTo(os) }
                    }
                }
                zis.closeEntry()
                entry = zis.nextEntry
            }
        }
    }
}

val clientSpecificationVersion = providers.gradleProperty("clientSpecificationVersion")
val clientSpecificationUrlTemplate = providers.gradleProperty("clientSpecificationUrlTemplate")

val clientSpecDir = layout.projectDirectory.dir("client-specification")

val fetchClientSpecification = tasks.register<FetchZip>("fetchClientSpecification") {
    group = "client-specification"
    description = "Downloads and unpacks client-specification"

    versionProp.set(clientSpecificationVersion)

    downloadUrl.set(clientSpecificationVersion.zip(clientSpecificationUrlTemplate) { v, tmpl ->
        String.format(tmpl, v)
    })
    destinationDir.set(clientSpecDir)
}
