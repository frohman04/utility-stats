package xyz.clieb.utilitystats

import java.io.File
import java.nio.file.Path

import scala.collection.mutable

import scopt.OptionParser
import xyz.clieb.utilitystats.util.Timed._

object Main {
  def main(args: Array[String]): Unit = {
    parser.parse(args, Options()) match {
      case Some(s) => new Main().run(
        s.electricPath.get.toPath,
        s.gasPath.get.toPath,
        s.smoothingDays.get,
        s.individual.get)
      case None =>
    }
  }

  case class Options(
      electricPath: Option[File] = None,
      gasPath: Option[File] = None,
      smoothingDays: Option[Int] = None,
      individual: Option[Boolean] = None)

  val parser: OptionParser[Options] = new OptionParser[Options]("utility-stats") {
    head("utility-stats", "0.1")

    opt[File]('e', "electric_file")
      .optional()
      .withFallback(() => new File("electric.csv"))
      .action((x, c) => c.copy(electricPath = Some(x)))
    opt[File]('g', "gas_file")
      .optional()
      .withFallback(() => new File("gas.csv"))
      .action((x, c) => c.copy(gasPath = Some(x)))
    opt[Int]('s', "smoothing_days")
      .optional()
      .withFallback(() => 14)
      .action((x, c) => c.copy(smoothingDays = Some(x)))
    opt[Boolean]('i', "individual")
      .optional()
      .withFallback(() => false)
      .action((x, c) => c.copy(individual = Some(x)))


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
  def run(electricPath: Path, gasPath: Path, smoothingDays: Int, individualGraphs: Boolean): Unit = {
    val tempMgr = new TempDataManager()
    if (individualGraphs) {
      timed("Drawing electricity usage graph") {
        new Grapher(new Measurements(electricPath, "Electricity", "kWh", TempType.HIGH), tempMgr)
          .render()
      }
      timed("Drawing gas usage graph") {
        new Grapher(new Measurements(gasPath, "Gas", "CCF", TempType.LOW), tempMgr)
          .render()
      }
    } else {
      timed("Drawing all util usage graph") {
        new AllUtilGrapher(
          new Measurements(electricPath, "Electricity", "kWh", TempType.AVERAGE).readFile(),
          new Measurements(gasPath, "Gas", "CCF", TempType.AVERAGE).readFile(),
          tempMgr,
          smoothingDays
        )
          .render()
      }
    }
  }
}
