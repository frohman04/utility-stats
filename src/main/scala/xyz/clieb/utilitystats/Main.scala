package xyz.clieb.utilitystats

import java.io.File
import java.nio.file.Path
import java.time.LocalDate

import co.theasi.plotly.writer.PlotFile
import com.github.tototoshi.csv.CSVReader
import scopt.OptionParser
import xyz.clieb.utilitystats.Closable._
import xyz.clieb.utilitystats.Timed._

import scala.collection.mutable
import scala.util.{Failure, Success}

object Main {
  def main(args: Array[String]): Unit = {
    parser.parse(args, Options()) match {
      case Some(s) => new Main().run(s.electricPath.get.toPath, s.gasPath.get.toPath)
      case None =>
    }
  }

  case class Options(
      electricPath: Option[File] = None,
      gasPath: Option[File] = None)

  val parser = new OptionParser[Options]("utility-stats") {
    head("utility-stats", "0.1")

    arg[File]("<electric_file>").action((x, c) =>
      c.copy(electricPath = Some(x)))
    arg[File]("<gas_file>").action((x, c) =>
      c.copy(gasPath = Some(x)))

    def validateFile(path: Option[File], name: String, isRequired: Boolean = true): Either[String, Unit] =
      if (path.isEmpty) {
        failure(s"Must specify the ${name} file")
      } else if (!path.get.exists()) {
        failure(s"The ${name} file does not exist: ${path.get.getAbsolutePath}")
      } else if (!path.get.isFile) {
        failure(s"The ${name} file is not a file: ${path.get.getAbsolutePath}")
      } else {
        success
      }

    /**
      * Combine the lefts of the provided Eithers if any are defined.
      */
    def eitherChain(eithers: Seq[Either[String, Unit]]): Either[String, Unit] = {
      val errors = mutable.ArrayBuffer[String]()
      for { either <- eithers } {
        if (either.isLeft) {
          errors += either.swap.getOrElse("")
        }
      }
      if (errors.nonEmpty) {
        Left[String, Unit](errors.mkString("\n"))
      } else {
        Right[String, Unit](Unit)
      }
    }

    checkConfig(c =>
      eitherChain(Seq(
        validateFile(c.electricPath, "electric"),
        validateFile(c.gasPath, "gas")
      )))
  }
}

class Main {
  def run(electricPath: Path, gasPath: Path): Unit = {
    timed("Drawing electricity usage graph") { graphElectric(electricPath) }
    timed("Drawing gas usage graph") { graphGas(gasPath) }
  }

  def graphElectric(path: Path): PlotFile = {
    val csvData = readFile(path, "kWh")

    null
  }

  def graphGas(path: Path): PlotFile = {
    val csvData = readFile(path, "CCF")

    null
  }

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
      }.toSeq
    } match {
      case Success(v) => v
      case Failure(e) => throw e
    }
  }

  implicit def orderedLocalDate: Ordering[LocalDate] = new Ordering[LocalDate] {
    def compare(x: LocalDate, y: LocalDate): Int = x compareTo y
  }
}

case class Measurement(
    date: LocalDate,
    amount: Float,
    units: String)
