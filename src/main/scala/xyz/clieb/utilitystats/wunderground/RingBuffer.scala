package xyz.clieb.utilitystats.wunderground

import com.typesafe.scalalogging.LazyLogging

import scala.collection.mutable

/**
  * A ring buffer that defaults unused elements to null.
  *
  * @param size the number of elements in the buffer
  *
  * @tparam T the type of object stored in the buffer
  */
private[wunderground] class RingBuffer[T](size: Int) extends LazyLogging {
  private val buffer = mutable.IndexedSeq.fill[T](size)(null.asInstanceOf[T])
  private var curr = 0
  private var next = 1

  /**
    * Add a new element to the buffer, overwriting the oldest element in the buffer if this would
    * cause the buffer to grow larger than the defined size.
    */
  def add(value: T): Unit = {
    curr = next
    next = (curr + 1) % buffer.length
    buffer(curr) = value
  }

  /**
    * Peek at the oldest object in the buffer.
    */
  def peekTail: T = buffer(next)
}
