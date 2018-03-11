package xyz.clieb.utilitystats

import java.time.LocalDate
import java.time.temporal.ChronoUnit

import org.apache.commons.math3.stat.regression.SimpleRegression
import plotly.Plotly._
import plotly.element._
import plotly.layout._
import plotly.{element, _}

class AllUtilGrapher(
    electricData: Seq[Measurement],
    gasData: Seq[Measurement],
    tempMgr: TempDataManager = new TempDataManager(),
    loessDays: Int = 14) {
  def render(): Unit = {
    implicit val localDateOrdering: Ordering[LocalDate] = Ordering.by(_.toEpochDay)
    val measDates = (electricData.map(_.date) ++ gasData.map(_.date))
      .distinct
      .sorted

    val dailyTempPlotData = tempMgr
      .dateRange(measDates.head, measDates.reverse.head)
      .map(date => (date, tempMgr.getTemp(date).mean))
    val loessTempPlotData = loessSimpleRegressionSeries(dailyTempPlotData, loessDays)

    val electricMeasPlotData = getPlotData(electricData)
    val gasMeasPlotData = getPlotData(gasData)

    Seq(
      dataToScatter(loessTempPlotData, s"Temp (F)", AxisReference.Y),
      dataToScatter(electricMeasPlotData, "Electric (kWh)", AxisReference.Y2),
      dataToScatter(gasMeasPlotData, "Gas (CCF)", AxisReference.Y3)
    ).plot(
      path = "all-utilities.html",
      title = s"All Utilities Usage per Day vs Average ${loessDays}-day Smoothed Temperature",
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

  private def loessSimpleRegressionSeries(data: Seq[(LocalDate, Float)], numDays: Int): (Seq[LocalDate], Seq[Float]) = {
    val baseDate = data.head._1

    val zippedOut = data
      .map { case (date: LocalDate, _) =>
        val lowerBound = date.minusDays(numDays / 2)
        val upperBound = date.plusDays((numDays - 1) / 2)

        val windowDays = data
          .filter { case (date: LocalDate, _) =>
            lowerBound.compareTo(date) <= 0 && date.compareTo(upperBound) <= 0
          }
          .map { case (date: LocalDate, temp: Float) =>
            (date.toEpochDay - baseDate.toEpochDay, temp)
          }

        val regression = new SimpleRegression()
        windowDays.foreach(day => regression.addData(day._1, day._2))
        (
          date,
          regression.predict(date.toEpochDay - baseDate.toEpochDay).toFloat
        )
      }

    (zippedOut.map(_._1), zippedOut.map(_._2))
  }

  private def dataToScatter(measPlotData: (Seq[LocalDate], Seq[Float]), name: String, yaxis: AxisReference): Scatter =
    Scatter(
      measPlotData._1.map(dt =>
        element.LocalDateTime(dt.getYear, dt.getMonthValue, dt.getDayOfMonth, 0, 0, 0)),
      measPlotData._2,
      name = name,
      mode = ScatterMode(ScatterMode.Lines),
      yaxis = yaxis
    )
}
