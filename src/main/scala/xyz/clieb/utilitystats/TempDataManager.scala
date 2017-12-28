package xyz.clieb.utilitystats

import com.typesafe.scalalogging.LazyLogging

import java.time.LocalDate

import scala.collection.mutable

import xyz.clieb.utilitystats.wunderground.{Client, Observation}

/**
  * Manager of temperature data retrieved from wunderground.  Strives for efficiency by caching
  * downloaded data on disk to minimise number of internet requests and by caching files in memory
  * as they are requested to minimize aoumt of disk access needed.
  */
class TempDataManager extends LazyLogging {
  private val client = new Client(enforceQuotas = false)
  private val cache = mutable.HashMap[LocalDate, Temp]()

  /**
    * Get the mean temperature in Farenheit for a given day.
    *
    * @param date the date to get the temperature for
    *
    * @return the mean temperature in Farenheit
    */
  def getTemp(date: LocalDate): Temp = {
    if (!cache.contains(date)) {
      cache(date) = fetchData(date)
    }
    cache(date)
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
    val temps = dateRange(fromDate, toDate)
        .map(date => selector(getTemp(date)))
        .toList
    temps.sum / temps.size
  }

  /**
    * Fetch the temperature data for a given date.  This data can come from disk cache or direct
    * from the WUnderground API.
    */
  private def fetchData(date: LocalDate): Temp = {
    val data = client.getHistorical(date)

    val temps = data.history.observations
        .map { case (obs: Observation) => obs.tempF }
        .filter { case (temp: Option[Float]) => temp.isDefined }
        .map { case (temp: Option[Float]) => temp.get }
    val min = temps.min
    val max = temps.max
    val mean = temps.sum / temps.size

    Temp(min, mean, max)
  }
}

case class Temp(
    min: Float,
    mean: Float,
    max: Float)
