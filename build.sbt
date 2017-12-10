name := "utility-stats"

version := "0.1"

scalaVersion := "2.12.4"

val slf4jVersion = "1.7.25"
val plotlyVersion = "0.3.3"

libraryDependencies ++= Seq(
  "org.slf4j" % "slf4j-api" % slf4jVersion,
  "org.slf4j" % "log4j-over-slf4j" % slf4jVersion,
  "org.slf4j" % "jcl-over-slf4j" % slf4jVersion,
  "org.slf4j" % "jul-to-slf4j" % slf4jVersion,
  "ch.qos.logback" % "logback-classic" % "1.2.3" % "runtime",
  "com.typesafe.scala-logging" %% "scala-logging" % "3.7.2",

  "com.github.scopt" %% "scopt" % "3.7.0",
  "org.apache.commons" % "commons-math3" % "3.6.1",
  "com.github.tototoshi" %% "scala-csv" % "1.3.5",
  "org.scalaj" % "scalaj-http_2.12" % "2.3.0",
  "org.plotly-scala" % "plotly-core_2.12" % plotlyVersion,
  "org.plotly-scala" % "plotly-render_2.12" % plotlyVersion
)
