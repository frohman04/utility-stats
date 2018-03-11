package xyz.clieb.utilitystats

import java.time.LocalDate
import java.time.temporal.ChronoUnit

import plotly.Plotly._
import plotly.element._
import plotly.layout._
import plotly.{element, _}

class AllUtilGrapher(
    electricData: Seq[Measurement],
    gasData: Seq[Measurement],
    tempMgr: TempDataManager = new TempDataManager()) {
  def render(): Unit = {
    implicit val localDateOrdering: Ordering[LocalDate] = Ordering.by(_.toEpochDay)
    val measDates = (electricData.map(_.date) ++ gasData.map(_.date))
      .distinct
      .sorted
    val tempPlotData = getTempData(measDates, tempMgr)

    val electricMeasPlotData = getPlotData(electricData)
    val gasMeasPlotData = getPlotData(gasData)

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
      dataToScatter(electricMeasPlotData, "Electric (kWh)", AxisReference.Y2),
      dataToScatter(gasMeasPlotData, "Gas (CCF)", AxisReference.Y3)
    )).plot(
      path = "all-utilities.html",
      title = s"All Utilities Usage per Day",
      xaxis = Axis(title = "Measurement Date"),
      yaxis = Axis(title = s"Avg Temp (F)"),
      yaxis2 = Axis(
        showticklabels = false,
        showgrid = false,
        overlaying = AxisAnchor.Reference(AxisReference.Y)
      ),
      yaxis3 = Axis(
        showticklabels = false,
        showgrid = false,
        overlaying = AxisAnchor.Reference(AxisReference.Y)
      ))
  }

  private def dataToScatter(measPlotData: (Seq[LocalDate], Seq[Float]), typ: String, yaxis: AxisReference): Scatter =
    Scatter(
      measPlotData._1.map(dt =>
        element.LocalDateTime(dt.getYear, dt.getMonthValue, dt.getDayOfMonth, 0, 0, 0)),
      measPlotData._2,
      name = typ,
      mode = ScatterMode(ScatterMode.Lines),
      yaxis = yaxis
    )

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
    *
    * @return (X data points, Y data points)
    */
  private def getTempData(
      utilData: Seq[LocalDate],
      tempMgr: TempDataManager): (Seq[LocalDate], Seq[Seq[Float]]) =
    (
      utilData.drop(1),
      utilData
        .zip(utilData.tail)
        .map { case (prev: LocalDate, curr: LocalDate) =>
          tempMgr.dateRange(prev, curr)
            .map(date => tempMgr.getTemp(date).mean)
        }
    )
}
