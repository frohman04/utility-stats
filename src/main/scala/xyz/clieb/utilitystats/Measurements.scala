package xyz.clieb.utilitystats

import com.github.tototoshi.csv.CSVReader

import java.nio.file.Path
import java.time.LocalDate

import scala.util.{Failure, Success}

import xyz.clieb.utilitystats.util.Closable.closable

/**
  * @param path the path of the CSV file to read
  * @param typ the type of utility being measured
  * @param units the units of the data being read
  * @param comparisonTempType the type of temperature to compare the measurements against
  */
class Measurements(path: Path, val typ: String, val units: String, val comparisonTempType: TempType) {
  /**
    * Read a CSV file of data, applying the provided units to each row.
    *
    * @return list of measurements read from the file
    */
  def readFile(): Seq[Measurement] = {
    closable(CSVReader.open(path.toFile)) { reader =>
      reader.iterator.map { case (values: Seq[String]) =>
        Measurement(LocalDate.parse(values(0)), values(1).toFloat, units)
      }.seq.toList
    } match {
      case Success(v) => v
      case Failure(e) => throw e
    }
  }
}


/**
  * A single meter reading.
  *
  * @param date the date of the meter reading
  * @param amount the amount of resources used since the last meter reading
  * @param units the units that the measurement is in
  */
case class Measurement(
    date: LocalDate,
    amount: Float,
    units: String)
