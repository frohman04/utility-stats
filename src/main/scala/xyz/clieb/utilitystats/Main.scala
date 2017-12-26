package xyz.clieb.utilitystats

import com.github.tototoshi.csv.CSVReader

import java.io.File
import java.nio.file.Path
import java.time.LocalDate
import java.time.temporal.ChronoUnit

import scala.collection.mutable
import scala.util.{Failure, Success}

import plotly._
import plotly.element
import plotly.element._
import plotly.layout._
import plotly.Plotly._
import scopt.OptionParser
import xyz.clieb.utilitystats.Closable._
import xyz.clieb.utilitystats.Timed._

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
    val tempMgr = new TempDataManager()
    timed("Drawing electricity usage graph") { graphElectric(electricPath, tempMgr) }
    timed("Drawing gas usage graph") { graphGas(gasPath, tempMgr) }
  }

  def graphElectric(path: Path, tempMgr: TempDataManager): Unit = {
    val measData = readFile(path, "kWh")
    val measPlotData = getPlotData(measData)
    val tempPlotData = getTempData(measData, tempMgr, "max")

    val measTrace = Scatter(
      values = measPlotData._1.map(dt =>
        element.LocalDateTime(dt.getYear, dt.getMonthValue, dt.getDayOfMonth, 0, 0, 0)),
      secondValues = measPlotData._2,
      mode = ScatterMode(ScatterMode.Lines),
      yaxis = AxisReference.Y2
    )
    val tempTrace = Scatter(
      values = tempPlotData._1.map(dt =>
        element.LocalDateTime(dt.getYear, dt.getMonthValue, dt.getDayOfMonth, 0, 0, 0)),
      secondValues = tempPlotData._2,
      mode = ScatterMode(ScatterMode.Lines)
    )
    Seq(measTrace, tempTrace).plot(
      path = "electric.html",
      openInBrowser = true,
      title = "Electricity Usage",
      xaxis = Axis(title = "Measurement Date"),
      yaxis = Axis(title = "Avg High Temp (F)"),
      yaxis2 = Axis(
        title = "kWh used / day",
        side = Side.Right
      ))
  }

  def graphGas(path: Path, tempMgr: TempDataManager): Unit = {
    val measData = readFile(path, "CCF")
    val measPlotData = getPlotData(measData)
    val tempPlotData = getTempData(measData, tempMgr, "min")

    val measTrace = Scatter(
      values = measPlotData._1.map(dt =>
        element.LocalDateTime(dt.getYear, dt.getMonthValue, dt.getDayOfMonth, 0, 0, 0)),
      secondValues = measPlotData._2,
      mode = ScatterMode(ScatterMode.Lines),
      yaxis = AxisReference.Y2
    )
    val tempTrace = Scatter(
      values = tempPlotData._1.map(dt =>
        element.LocalDateTime(dt.getYear, dt.getMonthValue, dt.getDayOfMonth, 0, 0, 0)),
      secondValues = tempPlotData._2,
      mode = ScatterMode(ScatterMode.Lines)
    )
    Seq(measTrace, tempTrace).plot(
      path = "gas.html",
      openInBrowser = true,
      title = "Gas Usage",
      xaxis = Axis(title = "Measurement Date"),
      yaxis = Axis(title = "Avg Los Temp (F)"),
      yaxis2 = Axis(
        title = "CCF used / day",
        side = Side.Right
      ))
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
      }.seq.toList
    } match {
      case Success(v) => v
      case Failure(e) => throw e
    }
  }

  /**
    * Get the plottable data series for a given measurement dataset.
    *
    * @param data the measurements to get plot data for
    *
    * @return (X data points, Y data points)
    */
  def getPlotData(data: Seq[Measurement]): (Seq[LocalDate], Seq[Float]) = {
    (
        data.drop(1).map(_.date),
        data
            .zip(data.tail)
            .map { case (prev: Measurement, curr: Measurement) =>
              val numDays = ChronoUnit.DAYS.between(prev.date, curr.date)
              curr.amount / numDays
            }
    )
  }

  /**
    * Get the temperature data to plot for a set of data for a utility.
    *
    * @param utilData the measurements for a utility's usage
    * @param tempMgr the temperature datamanager to query for temperature data
    * @param measurement one of 'min', 'mean', 'max'
    *
    * @return (X data points, Y data points)
    */
  def getTempData(utilData: Seq[Measurement], tempMgr: TempDataManager, measurement: String):
  (Seq[LocalDate], Seq[Float]) = {
    val getAvgTemp = measurement match {
      case "min" =>
        (fromDate: LocalDate, toDate: LocalDate) => tempMgr.getAvgMinTemp(fromDate, toDate)
      case "mean" =>
        (fromDate: LocalDate, toDate: LocalDate) => tempMgr.getAvgMeanTemp(fromDate, toDate)
      case "max" =>
        (fromDate: LocalDate, toDate: LocalDate) => tempMgr.getAvgMaxTemp(fromDate, toDate)
      case a: Any =>
        throw new IllegalArgumentException(
          s"Unknown measurement type: ${a}; expected one of min, mean, max")
    }

    (
        utilData.drop(1).map(_.date),
        utilData
            .zip(utilData.tail)
            .map { case (prev: Measurement, curr: Measurement) =>
              getAvgTemp(prev.date, curr.date)
            }
    )
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
