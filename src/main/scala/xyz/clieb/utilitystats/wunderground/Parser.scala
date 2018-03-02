package xyz.clieb.utilitystats.wunderground

import com.google.protobuf.timestamp.Timestamp

import org.json4s.{DefaultFormats, JValue}

import java.time.{ZoneId, ZonedDateTime}

/**
  * A parser object for common fields in Weather Underground response JSON objects.
  */
private[wunderground] class Parser {
  implicit val formats = DefaultFormats

  /**
    * Parse the "response" header.
    *
    * @param node node for the object assigned to the "response" key
    *
    * @return parsed response header
    */
  protected def parseResponseHeader(node: JValue): ResponseHeader = {
    val version = parseString(node \ "version").get
    val termsOfService = parseString(node \ "termsofService").get
    val features = (node \ "features").extract[Map[String, Int]]

    ResponseHeader(version, termsOfService, features)
  }

  /**
    * Parse the "date" JSON object into a Timestamp object.
    *
    * @param node node for the object assigned to a "date" key
    *
    * @return the date/time represented by the date object
    */
  protected def parseDate(node: JValue): Timestamp = {
    val year = parseInt(node \ "year").get
    val month = parseInt(node \ "mon").get
    val day = parseInt(node \ "mday").get
    val hour = parseInt(node \ "hour").get
    val minute = parseInt(node \ "min").get
    val zone = parseString(node \ "tzname").get

    val zdt = ZonedDateTime.of(
      year,
      month,
      day,
      hour,
      minute,
      0,
      0,
      ZoneId.of(zone))

    val ts = java.sql.Timestamp.from(zdt.toInstant)

    Timestamp(ts.getTime, ts.getNanos)
  }

  /**
    * Parse a string from a field, replacing "N/A' with None.
    */
  protected def parseString(node: JValue): Option[String] =
    safeParse[String](node, str => str, _ => true)

  /**
    * Parse an Int from a String field, replacing "N/A" and negative numbers with None.
    */
  protected def parseInt(node: JValue): Option[Int] =
    safeParse[Int](node, str => str.toInt, value => value != -999 && value != -9999)

  /**
    * Parse a Float from a String field, replacing "N/A" and negative numbers with None.
    */
  protected def parseFloat(node: JValue): Option[Float] =
    safeParse[Float](node, str => str.toFloat, value =>
      Math.abs(value - -999) >= 0.01 &&
        Math.abs(value - -9999) >= 0.01)

  /**
    * Parse a Boolean from a String field ("0" == false, "1" == true), replacing "N/A" and negative
    * numbers with None.
    */
  protected def parseBool(node: JValue): Option[Boolean] =
    safeParse[Int](node, str => str.toInt, value => value >= 0).map(_ == 1)

  /**
    * Safely parse a value field, turning "N/A" and certain values of the field into None.
    *
    * @param node the node to parse
    * @param parseValue function that takes the raw string extracted for the field and transforms it
    *                   into the correct type for the field
    * @param isValidValue function that determines if the parsed value for the field should be used
    *                     or replaced with None
    *
    * @tparam T the type of value to parse out of the String
    *
    * @return the parsed value or None
    */
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
