package xyz.clieb.utilitystats.wunderground

import org.json4s.{DefaultFormats, JValue}

import java.time.{ZoneId, ZonedDateTime}

private[wunderground] class Parser {
  implicit val formats = DefaultFormats

  def parseResponseHeader(node: JValue): ResponseHeader = {
    val version = parseString(node \ "version").get
    val termsOfService = parseString(node \ "termsofService").get
    val features = (node \ "features").extract[Map[String, Int]]

    ResponseHeader(version, termsOfService, features)
  }

  def parseDate(node: JValue): ZonedDateTime = {
    val year = parseInt(node \ "year").get
    val month = parseInt(node \ "mon").get
    val day = parseInt(node \ "mday").get
    val hour = parseInt(node \ "hour").get
    val minute = parseInt(node \ "min").get
    val zone = parseString(node \ "tzname").get

    ZonedDateTime.of(
      year,
      month,
      day,
      hour,
      minute,
      0,
      0,
      ZoneId.of(zone))
  }

  def parseString(node: JValue): Option[String] =
    safeParse[String](node, str => str, _ => true)

  def parseInt(node: JValue): Option[Int] =
    safeParse[Int](node, str => str.toInt, value => value >= 0)

  def parseFloat(node: JValue): Option[Float] =
    safeParse[Float](node, str => str.toFloat, value => value >= 0)

  def parseBool(node: JValue): Option[Boolean] =
    safeParse[Int](node, str => str.toInt, value => value >= 0).map(_ == 1)

  private def safeParse[T](
      node: JValue,
      parseValue: (String => T),
      isValidValue: (T => Boolean)): Option[T] = {
    val rawValue = node.extract[String]
    if (rawValue == "N/A") {
      None
    } else {
      val parsedValue = parseValue(rawValue)
      if (isValidValue(parsedValue)) {
        Some(parsedValue)
      } else {
        None
      }
    }
  }
}
