name := "utility-stats"

version := "0.1"

scalaVersion := "2.12.6"

val slf4jVersion = "1.7.25"
val plotlyVersion = "0.4.2"
val json4sVersion = "3.6.1"

libraryDependencies ++= Seq(
  "org.slf4j" % "slf4j-api" % slf4jVersion,
  "org.slf4j" % "log4j-over-slf4j" % slf4jVersion,
  "org.slf4j" % "jcl-over-slf4j" % slf4jVersion,
  "org.slf4j" % "jul-to-slf4j" % slf4jVersion,
  "ch.qos.logback" % "logback-classic" % "1.2.3" % "runtime",
  "com.typesafe.scala-logging" %% "scala-logging" % "3.9.0",

  "com.github.scopt" %% "scopt" % "3.7.0",
  "com.github.tototoshi" %% "scala-csv" % "1.3.5",
  "org.scalaj" %% "scalaj-http" % "2.4.1",
  "org.json4s" %% "json4s-core" % json4sVersion,
  "org.json4s" %% "json4s-jackson" % json4sVersion,
  "org.plotly-scala" %% "plotly-core" % plotlyVersion,
  "org.plotly-scala" %% "plotly-render" % plotlyVersion,
  "com.thesamet.scalapb" %% "scalapb-runtime" % scalapb.compiler.Version.scalapbVersion % "protobuf",
  "org.apache.commons" % "commons-compress" % "1.18",
  "org.apache.commons" % "commons-math3" % "3.6.1"
)

PB.targets in Compile := Seq(
  scalapb.gen() -> (sourceManaged in Compile).value
)
