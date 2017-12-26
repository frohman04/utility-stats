package xyz.clieb.utilitystats.wunderground

import com.esotericsoftware.kryo.io.{Input, Output}
import com.twitter.chill.{KryoInstantiator, ScalaKryoInstantiator}
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

import xyz.clieb.utilitystats.Closable.closable

class Client(storageDir: Path = Paths.get("wunderground_cache")) extends LazyLogging {
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

  private val kryo = new ScalaKryoInstantiator().newKryo()
  kryo.register(classOf[HistoryResponse])
  kryo.register(classOf[ResponseHeader])
  kryo.register(classOf[History])
  kryo.register(classOf[Observation])

  def getHistorical(date: LocalDate): HistoryResponse = {
    if (LocalDate.from(date).equals(LocalDate.now())) {
      throw new IllegalArgumentException("Cannot query history for today")
    }

    val dateStr = date.format(DateTimeFormatter.ofPattern("yyyyMMdd"))
    val cacheFile = Paths.get(historyStorageDir.toString, s"${dateStr}.kryo.gz")

    if (cacheFile.toFile.exists()) {
      closable(new Input(new GzipCompressorInputStream(new FileInputStream(cacheFile.toFile)))) { input =>
        kryo.readObject(input, classOf[HistoryResponse])
      } match {
        case Success(r) => r
        case Failure(e) => throw e
      }
    } else {
      val url = s"${Client.apiBase}/history_${dateStr}/q/MA/Billerica.json"
      val response = historyParser.parseHistoryResponse(apiCall(url))

      closable(new Output(new GzipCompressorOutputStream(new FileOutputStream(cacheFile.toFile)))) { output =>
        kryo.writeObject(output, response)
      }

      response
    }
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
