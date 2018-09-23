package xyz.clieb.utilitystats

import java.time.LocalDate
import java.time.temporal.ChronoUnit

import plotly.Plotly._
import plotly.element._
import plotly.layout._
import plotly.{element, _}

/**
  * Utility for graphing utility usage against the temperature over time.
  *
  * @param measurements the utility measurements to graph
  * @param tempMgr the temperature data to graph
  */
class Grapher(
    measurements: Measurements,
    tempMgr: TempDataManager = new TempDataManager()) {
  def render(): Unit = {
    val measData = measurements.readFile()
    val measPlotData = getPlotData(measData)
    val tempPlotData = getTempData(measData, tempMgr, measurements.comparisonTempType)
    val tempTypeName = measurements.comparisonTempType.toString
        .toLowerCase()
        .split('_')
        .map(_.capitalize)
        .mkString(" ")

    val boxes: Seq[Box] = tempPlotData._1.zip(tempPlotData._2)
        .map { case (date: LocalDate, temps: Seq[Float]) =>
          Box(
            y = temps,
            name = date.toString,
            orientation = Orientation.Vertical,
            boxpoints = BoxPoints.False,
            showlegend = false
          )
        }

    (boxes ++ Seq(
      Scatter(
        measPlotData._1.map(dt =>
          element.LocalDateTime(dt.getYear, dt.getMonthValue, dt.getDayOfMonth, 0, 0, 0)),
        measPlotData._2,
        name = s"${measurements.typ} Usage",
        mode = ScatterMode(ScatterMode.Lines),
        yaxis = AxisReference.Y2
      )
    )).plot(
      path = s"${measurements.typ.toLowerCase}.html",
      openInBrowser = true,
      title = s"${measurements.typ} Usage",
      xaxis = Axis(title = "Measurement Date"),
      yaxis = Axis(title = s"Avg ${tempTypeName} Temp (F)"),
      yaxis2 = Axis(
        title = s"${measurements.units} used / day",
        side = Side.Right,
        overlaying = AxisAnchor.Reference(AxisReference.Y)
      ))
  }

  /**
    * Get the plottable data series for a given measurement dataset.
    *
    * @param data the measurements to get plot data for
    *
    * @return (X data points, Y data points)
    */
  private def getPlotData(data: Seq[Measurement]): (Seq[LocalDate], Seq[Float]) = {
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
    * @param measurement the temperature data point to collect
    *
    * @return (X data points, Y data points)
    */
  private def getTempData(
      utilData: Seq[Measurement],
      tempMgr: TempDataManager,
      measurement: TempType): (Seq[LocalDate], Seq[Seq[Float]]) = {
    val getTemp = measurement match {
      case TempType.LOW =>
        (temp: Temp) => temp.min
      case TempType.AVERAGE =>
        (temp: Temp) => temp.mean
      case TempType.HIGH =>
        (temp: Temp) => temp.max
    }

    (
        utilData.drop(1).map(_.date),
        utilData
            .zip(utilData.tail)
            .map { case (prev: Measurement, curr: Measurement) =>
              tempMgr.dateRange(prev.date, curr.date)
                    .map(date => getTemp(tempMgr.getTemp(date).get))
            }
    )
  }
}
