package xyz.clieb.utilitystats.wunderground

import java.time.ZonedDateTime

case class HistoryResponse(response: ResponseHeader, history: History)

case class History(date: ZonedDateTime, observations: Seq[Observation])

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
