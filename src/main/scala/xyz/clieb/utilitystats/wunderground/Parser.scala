package xyz.clieb.utilitystats.wunderground

import org.json4s.{DefaultFormats, JValue}

import java.time.{ZoneId, ZonedDateTime}

private[wunderground] class Parser {
  implicit val formats = DefaultFormats

  def parseResponseHeader(node: JValue): ResponseHeader = {
    val version = (node \ "version").extract[String]
    val termsOfService = (node \ "termsofService").extract[String]
    val features = (node \ "features").extract[Map[String, Int]]

    ResponseHeader(version, termsOfService, features)
  }

  def parseDate(node: JValue): ZonedDateTime = {
    val year = (node \ "year").extract[String].toInt
    val month = (node \ "mon").extract[String].toInt
    val day = (node \ "mday").extract[String].toInt
    val hour = (node \ "hour").extract[String].toInt
    val minute = (node \ "min").extract[String].toInt
    val zone = (node \ "tzname").extract[String]

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

  def parseInt(node: JValue): Int = node.extract[String].toInt

  def parseFloat(node: JValue): Float = node.extract[String].toFloat

  def parseBool(node: JValue): Boolean = node.extract[String].toInt == 1
}
