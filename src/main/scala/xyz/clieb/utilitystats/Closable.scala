package xyz.clieb.utilitystats

import scala.util.{Failure, Success, Try}

/**
  * Scala implementation of try-with-resources.  Adapted from
  * {@link https://www.phdata.io/try-with-resources-in-scala/}.
  */
object Closable {
  /**
    * Scala implementation of try-with-resources.  Adapted from
    * {@link https://www.phdata.io/try-with-resources-in-scala/}.
    *
    * @param resource the resource to close after exiting block
    * @param doWork the code that will operate on the resource
    *
    * @tparam A the type of the resource being managed
    * @tparam B the type of the output of the block
    *
    * @return the return value of doWork
    */
  def closable[A <: AutoCloseable, B](resource: A)(doWork: A => B): Try[B] = {
    var err: Throwable = null
    try {
      Success(doWork(resource))
    } catch {
      case e: Throwable =>
        err = e
        Failure(e)
    } finally {
      if (resource != null) {
        try {
          resource.close()
        } catch {
          case e: Throwable =>
            if (err != null) {
              err.addSuppressed(e)
            }
            Failure(err)
        }
      }
    }
  }
}
