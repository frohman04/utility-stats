package xyz.clieb.utilitystats.wunderground

import com.typesafe.scalalogging.LazyLogging

import org.json4s._
import org.json4s.jackson.JsonMethods._

import java.time.LocalDateTime
import java.time.format.DateTimeFormatter

import scalaj.http.Http

class Client extends LazyLogging {
  private val historyParser = new HistoryResponseParser()

  def getHistorical(date: LocalDateTime): HistoryResponse = {
    val dateStr = date.format(DateTimeFormatter.ofPattern("yyyyMMdd"))
    val url = s"${Client.apiBase}/history_${dateStr}/q/MA/Billerica.json"

    logger.info(s"Getting data for ${date} using ${url}")
    val response = Http(url).asString

    historyParser.parseHistoryResponse(parse(response.body))
  }
}

object Client {
  private val apiKey = "effc0f07ae9ec505"
  private val requestsPerMinute = 10
  private val requestsPerDay = 500

  private val apiBase = s"http://api.wunderground.com/api/${apiKey}"
}
