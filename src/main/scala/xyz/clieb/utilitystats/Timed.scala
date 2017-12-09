package xyz.clieb.utilitystats

import com.typesafe.scalalogging.{LazyLogging, Logger}

/**
  * Utilities for timing the exeution of code.
  */
object Timed extends LazyLogging {
  /**
    * Run a block of code and log how long that block took to run.
    *
    * @param message the message to log along with the time
    * @param logger the logger to use to log the time
    * @param doWork the code to time
    *
    * @tparam T the return type of doWork
    *
    * @return the output of doWork
    */
  def timed[T](message: String, logger: Logger = this.logger)(doWork: =>T): T = {
    val startTime = System.nanoTime()

    logger.info(s"Start: ${message}")

    val out: T = doWork

    logger.info(s"End:   ${message}: ${(System.nanoTime() - startTime) * 1e-9} s")

    out
  }
}
