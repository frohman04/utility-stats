package xyz.clieb.utilitystats

import com.github.tototoshi.csv.CSVReader
import com.typesafe.scalalogging.LazyLogging

import java.io.{BufferedWriter, FileWriter}
import java.nio.file.{Path, Paths}
import java.time.LocalDate
import java.time.format.DateTimeFormatter

import scala.collection.mutable
import scala.util.{Failure, Success}
import scalaj.http.Http

import xyz.clieb.utilitystats.Closable._

/**
  * Manager of temperature data retrieved from wunderground.  Strives for efficiency by caching
  * downloaded data on disk to minimise number of internet requests and by caching files in memory
  * as they are requested to minimize aoumt of disk access needed.
  *
  * @param storageDir the directory on dist to cache downloded files in
  */
class TempDataManager(storageDir: Path = Paths.get("temp_data")) extends LazyLogging {
  if (!storageDir.toFile.exists()) {
    storageDir.toFile.mkdirs()
  }
  private val cache = mutable.HashMap[String, Map[LocalDate, Temp]]()

  /**
    * Get the mean temperature in Farenheit for a given day.
    *
    * @param date the date to get the temperature for
    *
    * @return the mean temperature in Farenheit
    */
  def getTemp(date: LocalDate): Temp = {
    val key = getKey(date)
    if (!cache.contains(key)) {
      loadData(date)
    }
    cache(key)(date)
  }

  /**
    * Get the average temperature over a range of days, using each day's minimum temperature in
    * Farenheit as the data point to average.
    *
    * @param fromDate the first date in the range (inclusive)
    * @param toDate the last date in the range (exclusive)
    *
    * @return the average temperature in Farenheit
    */
  def getAvgMinTemp(fromDate: LocalDate, toDate: LocalDate): Float =
    getAvgTemp(fromDate, toDate, (x: Temp) => x.min)

  /**
    * Get the average temperature over a range of days, using each day's mean temperature in
    * Farenheit as the data point to average.
    *
    * @param fromDate the first date in the range (inclusive)
    * @param toDate the last date in the range (exclusive)
    *
    * @return the average temperature in Farenheit
    */
  def getAvgMeanTemp(fromDate: LocalDate, toDate: LocalDate): Float =
    getAvgTemp(fromDate, toDate, (x: Temp) => x.mean)

  /**
    * Get the average temperature over a range of days, using each day's maximum temperature in
    * Farenheit as the data point to average.
    *
    * @param fromDate the first date in the range (inclusive)
    * @param toDate the last date in the range (exclusive)
    *
    * @return the average temperature in Farenheit
    */
  def getAvgMaxTemp(fromDate: LocalDate, toDate: LocalDate): Float =
    getAvgTemp(fromDate, toDate, (x: Temp) => x.max)

  /**
    * Get the average temperature over a range of days, using each day's mean temperature in
    * Farenheit as the data point to average.
    *
    * @param fromDate the first date in the range (inclusive)
    * @param toDate the last date in the range (exclusive)
    * @param selector function that translates a Temp object into the desired temperature
    *
    * @return the average temperature in Farenheit
    */
  private def getAvgTemp(fromDate: LocalDate, toDate: LocalDate, selector: (Temp) => Float): Float = {
    /**
      * Generate a range of dates across a range.
      *
      * @param startDate the first date in the range (inclusive)
      * @param endDate the last date in the range (exclusive)
      */
    def dateRange(startDate: LocalDate, endDate: LocalDate): Stream[LocalDate] = {
      def nextDate(date: LocalDate, lastDate: LocalDate): Stream[LocalDate] =
        if (date.compareTo(lastDate) == 0) {
          Stream.empty
        } else {
          date #:: nextDate(date.plusDays(1), endDate)
        }

      nextDate(startDate, endDate)
    }

    val temps = dateRange(fromDate, toDate)
        .map(date => selector(getTemp(date)))
        .toList
    temps.sum / temps.size
  }

  /**
    * Get the key into the cache for a given date.
    *
    * @param date the date to get the key for
    *
    * @return the cache key
    */
  private def getKey(date: LocalDate): String =
    date.format(DateTimeFormatter.ofPattern("yyyy-MM"))

  /**
    * Load data from disk into cache for a given date.  May also cause data for other dates to be
    * loaded at the same time.  If the data is not yet on disk, then download it.
    *
    * @param date the date who's data should be loaded
    */
  private def loadData(date: LocalDate): Unit = {
    val dataFileName = Paths.get(storageDir.toString, s"${date.getYear}-${date.getMonthValue}.csv")

    // download the data if we don't have any data for the month
    if (!dataFileName.toFile.exists()) {
      downloadData(date.getYear, date.getMonthValue, dataFileName)
    }

    // load the data from file
    val data = loadDataFromDisk(dataFileName)

    // if the month doesn't have complete data, redownload it again
    val lastDay = data.keySet.max
    val firstDay = LocalDate.of(lastDay.getYear, lastDay.getMonthValue, 1)
    val expectedLastDay = firstDay.plusMonths(1).minusDays(1)
    val freshenedData = if (lastDay.compareTo(expectedLastDay) < 0 &&
        lastDay.compareTo(LocalDate.now()) != 0) {
      downloadData(date.getYear, date.getMonthValue, dataFileName)
      loadDataFromDisk(dataFileName)
    } else {
      data
    }

    // save the downloaded data in memory
    cache(getKey(date)) = freshenedData
  }

  /**
    * Load the data contained in a downloaded file from disk.
    *
    * @param path the file to load
    *
    * @return temperature in Farenheit for each day of the month
    */
  private def loadDataFromDisk(path: Path): Map[LocalDate, Temp] =
    closable(CSVReader.open(path.toFile)) { reader =>
      reader.iteratorWithHeaders
          .map { case (row: Map[String, String]) =>
              val timeStr = if (row.contains("EST")) {
                row("EST")
              } else {
                row("EDT")
              }
              val date = LocalDate.parse(timeStr)
              val minTemp = row("Min TemperatureF").toInt
              val maxTemp = row("Max TemperatureF").toInt
              val meanTemp = if (row("Mean TemperatureF") == "") {
                (maxTemp + minTemp) / 2
              } else {
                row("Mean TemperatureF").toInt
              }
            (date, Temp(minTemp, meanTemp, maxTemp))
          }
          .seq
          .toList
    } match {
      case Success(v) => v.toMap
      case Failure(e) => throw e
    }

  /**
    * Download the temperature data for a given month.
    *
    * @param year the four digit year to download data for
    * @param month the numerical month (January = 1) to download data for
    * @param outPath the file to save the data into
    */
  private def downloadData(year: Int, month: Int, outPath: Path): Unit = {
    logger.info(s"Downloading data for ${year}-${month}")
    val url = getDataUrl(year, month)
    logger.debug(s"\tURL: ${url}")
    logger.debug(s"\tOut File: ${outPath}")
    val response = Http(url).asString
    closable(new BufferedWriter(new FileWriter(outPath.toFile))) { writer =>
      writer.write(response.body.replace("<br />", ""))
    }
  }

  private def getDataUrl(year: Int, month: Int) =
    s"http://www.wunderground.com/history/airport/KBED/${year}/${month}/1/MonthlyHistory.html?format=1"

  implicit def orderedLocalDate: Ordering[LocalDate] = new Ordering[LocalDate] {
    def compare(x: LocalDate, y: LocalDate): Int = x compareTo y
  }
}

case class Temp(
    min: Float,
    mean: Float,
    max: Float)
