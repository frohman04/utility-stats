package xyz.clieb.utilitystats.wunderground

import com.typesafe.scalalogging.LazyLogging

import org.json4s._
import org.json4s.jackson.JsonMethods._

import java.time.LocalDateTime
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit

import scalaj.http.Http

class Client extends LazyLogging {
  private val historyParser = new HistoryResponseParser()
  private val requestPerMinuteTracker = new RingBuffer[LocalDateTime](Client.requestsPerMinute)
  private var totalRequestsMade = 0

  def getHistorical(date: LocalDateTime): HistoryResponse = {
    val dateStr = date.format(DateTimeFormatter.ofPattern("yyyyMMdd"))
    val url = s"${Client.apiBase}/history_${dateStr}/q/MA/Billerica.json"
    historyParser.parseHistoryResponse(apiCall(url))
  }

  private def apiCall(url: String): JValue = {
    totalRequestsMade += 1
    if (totalRequestsMade > Client.requestsPerDay) {
      throw new RuntimeException("Too many requests made for today")
    }

    val now = LocalDateTime.now()
    if (requestPerMinuteTracker.peekTail != null &&
        now.minusMinutes(1).isBefore(requestPerMinuteTracker.peekTail)) {
      logger.info("Requests per minute exceeded, sleeping")
      Thread.sleep(requestPerMinuteTracker.peekTail.until(now, ChronoUnit.MILLIS))
    }

    logger.info(s"Calling Wunderground: ${url}")
    val response = Http(url).asString

    parse(response.throwError.body)
  }
}

object Client {
  private val apiKey = "effc0f07ae9ec505"
  private val requestsPerMinute = 10
  private val requestsPerDay = 500

  private val apiBase = s"http://api.wunderground.com/api/${apiKey}"
}
