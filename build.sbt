name := "utility-stats"

version := "0.1"

scalaVersion := "2.12.4"

val slf4jVersion = "1.7.25"

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

  // need to build by-hand to get version that works w/ Scala 2.12
  //   * set the SBT version to 0.13.16
  //   * add 2.12.3 as additional cross version
  //   * upgrade to scalaj-http 2.3.0
  //   * upgrade to json4s-native 3.5.0
  //   * upgrade to scalatest 3.0.4
  "co.theasi" %% "plotly" % "0.2.0"
)
