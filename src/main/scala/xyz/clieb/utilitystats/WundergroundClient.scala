package xyz.clieb.utilitystats

import com.typesafe.scalalogging.LazyLogging

import org.json4s._
import org.json4s.jackson.JsonMethods._

import java.time.format.DateTimeFormatter
import java.time.{LocalDateTime, ZoneId, ZonedDateTime}

import scalaj.http.Http

class WundergroundClient extends LazyLogging {
  implicit val formats = DefaultFormats

  def getHistorical(date: LocalDateTime): HistoryResponse = {
    val dateStr = date.format(DateTimeFormatter.ofPattern("yyyyMMdd"))
    val url = s"${WundergroundClient.apiBase}/history_${dateStr}/q/MA/Billerica.json"

    logger.info(s"Getting data for ${date} using ${url}")
    val response = Http(url).asString

    parseHistoryResponse(parse(response.body))
  }

  private def parseHistoryResponse(node: JValue): HistoryResponse = {
    val header = parseResponseHeader(node \ "response")
    val history = parseHistory(node \ "history")

    HistoryResponse(header, history)
  }

  private def parseResponseHeader(node: JValue): ResponseHeader = {
    val version = (node \ "version").extract[String]
    val termsOfService = (node \ "termsofService").extract[String]
    val features = (node \ "features").extract[Map[String, Int]]

    ResponseHeader(version, termsOfService, features)
  }

  private def parseHistory(node: JValue): History = {
    val date = parseDate(node \ "date")
    val observations = parseObservations(node \ "observations")

    History(date, observations)
  }

  private def parseObservations(node: JValue): Seq[Observation] =
    node.children.map(parseObservation)

  private def parseObservation(node: JValue): Observation = {
    val date = parseDate(node \ "date")
    val tempF = parseFloat(node \ "tempi")
    val dewPtF = parseFloat(node \ "dewpti")
    val humidity = parseInt(node \ "hum")
    val windSpeedMph = parseFloat(node \ "wspdi")
    val windGustMph = parseFloat(node \ "wgusti")
    val windDirDeg = parseInt(node \ "wdird")
    val visibilityMiles = parseFloat(node \ "visi")
    val pressureInHg = parseFloat(node \ "pressurei")
    val windChillF = parseFloat(node \ "windchilli")
    val heatIndexF = parseFloat(node \ "heatindexi")
    val precipitationIn = parseFloat(node \ "precipi")
    val conditions = (node \ "conds").extract[String]
    val fog = parseBool(node \ "fog")
    val rain = parseBool(node \ "rain")
    val snow = parseBool(node \ "snow")
    val hail = parseBool(node \ "hail")
    val thunder = parseBool(node \ "thunder")
    val tornado = parseBool(node \ "tornado")

    Observation(
      date,
      tempF,
      dewPtF,
      humidity,
      windSpeedMph,
      windGustMph,
      windDirDeg,
      visibilityMiles,
      pressureInHg,
      windChillF,
      heatIndexF,
      precipitationIn,
      conditions,
      fog,
      rain,
      snow,
      hail,
      thunder,
      tornado)
  }

  private def parseDate(node: JValue): ZonedDateTime = {
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

  private def parseInt(node: JValue): Int = node.extract[String].toInt

  private def parseFloat(node: JValue): Float = node.extract[String].toFloat

  private def parseBool(node: JValue): Boolean = node.extract[String].toInt == 1
}

object WundergroundClient {
  private val apiKey = "effc0f07ae9ec505"
  private val requestsPerMinute = 10
  private val requestsPerDay = 500

  private val apiBase = s"http://api.wunderground.com/api/${apiKey}"
}

case class ResponseHeader(version: String, termsOfService: String, features: Map[String, Int])

case class Observation(
    date: ZonedDateTime,
    tempF: Float,
    dewPtF: Float,
    humidity: Int,
    windSpeedMph: Float,
    windGustMph: Float,
    windDirDeg: Int,
    visibilityMiles: Float,
    pressureInHg: Float,
    windChillF: Float,
    heatIndexF: Float,
    precipitationIn: Float,
    conditions: String,
    fog: Boolean,
    rain: Boolean,
    snow: Boolean,
    hail: Boolean,
    thunder: Boolean,
    tornado: Boolean)

case class History(date: ZonedDateTime, observations: Seq[Observation])

case class HistoryResponse(response: ResponseHeader, history: History)
