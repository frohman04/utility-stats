package xyz.clieb.utilitystats.wunderground

import org.json4s.JValue

private[wunderground] class HistoryResponseParser extends Parser {
  /**
    * Parse a response from /history_yyyyMMdd into an object.
    */
  def parseHistoryResponse(node: JValue): HistoryResponse = {
    val header = parseResponseHeader(node \ "response")
    val history = parseHistory(node \ "history")

    HistoryResponse(header, history)
  }

  /**
    * Parse the "history" object into a History object.
    */
  private def parseHistory(node: JValue): History = {
    val date = parseDate(node \ "date")
    val observations = parseObservations(node \ "observations")

    History(date, observations)
  }

  /**
    * Parse a list of "observation" objects into Observation objects.
    */
  private def parseObservations(node: JValue): Seq[Observation] =
    node.children.map(parseObservation)

  /**
    * Parse an "observation" object into an Observation object.
    */
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
}
