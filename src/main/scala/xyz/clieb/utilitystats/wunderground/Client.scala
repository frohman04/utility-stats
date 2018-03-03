package xyz.clieb.utilitystats.wunderground

import com.typesafe.scalalogging.LazyLogging

import org.apache.commons.compress.compressors.gzip.{GzipCompressorInputStream, GzipCompressorOutputStream}
import org.json4s._
import org.json4s.jackson.JsonMethods._

import java.io.{FileInputStream, FileOutputStream}
import java.nio.file.{Path, Paths}
import java.time.format.DateTimeFormatter
import java.time.temporal.ChronoUnit
import java.time.{LocalDate, LocalDateTime}

import scala.util.{Failure, Success, Try}
import scalaj.http.{Http, HttpStatusException}

import xyz.clieb.utilitystats.util.Closable.closable

/**
  * A client for the Weather Underground API.
  *
  * @param enforceQuotas if true, stop when quotas reached
  * @param storageDir the directory to store cached responses in
  */
class Client(
    enforceQuotas: Boolean = true,
    storageDir: Path = Paths.get("wunderground_cache")) extends LazyLogging {
  if (!storageDir.toFile.exists()) {
    storageDir.toFile.mkdirs()
  }
  private val historyStorageDir = Paths.get(storageDir.toString, "history")
  if (!historyStorageDir.toFile.exists()) {
    historyStorageDir.toFile.mkdirs()
  }

  private val historyParser = new HistoryResponseParser()
  private val requestPerMinuteTracker = new RingBuffer[LocalDateTime](Client.requestsPerMinute)
  private var totalRequestsMade = 0

  /**
    * Get the weather conditions throughout the day for a day in the past.
    *
    * If this date's conditions have been retrieved in the past, then pull the cached response from
    * disk since it is highly unlikely that the conditions have changed after the day ended.
    *
    * @param date the date the pull history for
    *
    * @return the historical weather conditions
    */
  def getHistorical(date: LocalDate): HistoryResponse = {
    if (LocalDate.from(date).equals(LocalDate.now())) {
      throw new IllegalArgumentException("Cannot query history for today")
    }

    val dateStr = date.format(DateTimeFormatter.ofPattern("yyyyMMdd"))
    val cacheFile = Paths.get(historyStorageDir.toString, s"${dateStr}.proto.gz")

    if (cacheFile.toFile.exists()) {

      closable(new GzipCompressorInputStream(new FileInputStream(cacheFile.toFile))) { input =>
        HistoryResponse.parseFrom(input)
      } match {
        case Success(r) => r
        case Failure(e) => throw e
      }
    } else {
      val url = s"${Client.apiBase}/history_${dateStr}/q/MA/Billerica.json"
      val response = historyParser.parseHistoryResponse(apiCall(url))

      closable(new GzipCompressorOutputStream(new FileOutputStream(cacheFile.toFile))) { output =>
        response.writeTo(output)
      }

      response
    }
  }

  /**
    * Make a call to the Weather Underground API.
    *
    * Supports limited retries since the API has moments of flakyness.
    *
    * Also supports primitive rate limiting at the minute and day resolution.  A day is defined as
    * the lifetime of this program and is not tracked across invocations.
    *
    * @param url the URL on the API to call
    * @param retries the number of times to retry the request before deeming it a failure
    *
    * @return the parsed JSON body of the request
    */
  private def apiCall(url: String, retries: Int = 2): JValue = {
    if (enforceQuotas) {
      totalRequestsMade += 1
      if (totalRequestsMade > Client.requestsPerDay) {
        throw new RuntimeException("Too many requests made for today")
      }

      val now = LocalDateTime.now()
      if (requestPerMinuteTracker.peekTail != null &&
          now.minusMinutes(1).isBefore(requestPerMinuteTracker.peekTail)) {
        val sleep = requestPerMinuteTracker.peekTail.until(now, ChronoUnit.MILLIS)
        logger.info(s"Requests per minute exceeded, sleeping ${sleep}")
        Thread.sleep(sleep)
      }
      requestPerMinuteTracker.add(now)
    }

    logger.info(s"Calling Wunderground: ${url}")
    val response = Http(url).asString

    try {
      val rawBody = Try(response.throwError.body) match {
        case Success(body) => body
        case Failure(e: HttpStatusException) =>
          val headers = response.headers
              .toSeq
              .flatMap { case (key: String, values: Seq[String]) =>
                values.map { case (value: String) => (key, value) }
              }
              .map { case (key: String, value: String) => s"${key}: ${value}" }
              .mkString("\n")
          logger.error(s"${e.code} ${e.statusLine}\n${headers}\n\n${response.body}")

          throw e
        case Failure(e) =>
          throw e
      }

      Try(parse(rawBody)) match {
        case Success(json) => json
        case Failure(e) =>
          logger.error(rawBody, e)

          throw e
      }
    } catch {
      case e: Throwable =>
        // retry the request if it fails for any reason and there are retry attempts remaining
        if (retries > 0) {
          val remaining = retries - 1
          logger.warn(s"API call failed, retrying (${remaining} remaining)")
          apiCall(url, remaining)
        } else {
          throw e
        }
    }
  }
}

object Client {
  private val apiKey = "effc0f07ae9ec505"
  private val requestsPerMinute = 10
  private val requestsPerDay = 500

  private val apiBase = s"http://api.wunderground.com/api/${apiKey}"
}
