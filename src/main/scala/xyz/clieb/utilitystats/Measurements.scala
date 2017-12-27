package xyz.clieb.utilitystats

import com.github.tototoshi.csv.CSVReader

import java.nio.file.Path
import java.time.LocalDate

import scala.util.{Failure, Success}

import xyz.clieb.utilitystats.util.Closable.closable

class Measurements {
  /**
    * Read a CSV file of data, applying the provided units to each row.
    *
    * @param path the path of the CSV file to read
    * @param units the units of the data being read
    *
    * @return list of measurements read from the file
    */
  def readFile(path: Path, units: String): Seq[Measurement] = {
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
