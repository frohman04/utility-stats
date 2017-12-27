package xyz.clieb.utilitystats.wunderground

import java.time.ZonedDateTime

case class HistoryResponse(response: ResponseHeader, history: History)

case class History(date: ZonedDateTime, observations: Seq[Observation])

case class Observation(
    date: ZonedDateTime,
    tempF: Option[Float],
    dewPtF: Option[Float],
    humidity: Option[Int],
    windSpeedMph: Option[Float],
    windGustMph: Option[Float],
    windDirDeg: Option[Int],
    visibilityMiles: Option[Float],
    pressureInHg: Option[Float],
    windChillF: Option[Float],
    heatIndexF: Option[Float],
    precipitationIn: Option[Float],
    conditions: String,
    fog: Option[Boolean],
    rain: Option[Boolean],
    snow: Option[Boolean],
    hail: Option[Boolean],
    thunder: Option[Boolean],
    tornado: Option[Boolean])
