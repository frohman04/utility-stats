#!/usr/bin/env python

import argparse
import csv
import datetime
import matplotlib.pyplot as plt


class Measurement(object):
    """A single meter reading."""

    def __init__(self, date, amount, units):
        """Create a new Measurement.

        Args:
            date (datetime.date): the date of the meter reading
            amount (float): the amount of resources used since the last meter
                    reading
            units (str): the units that the measurement is in
        """

        self._date = date
        self._amount = amount
        self._units = units

    def get_date(self):
        """Get the date of the meter reading."""

        return self._date

    def get_amount(self):
        """Get the amount of resources used since the last meter reading."""

        return self._amount

    def get_units(self):
        """Return:
            (str): The units that the measurement is in
        """

        return self._units

    def __repr__(self):
        return 'Measurement(%s, %s, %s)' % (
                self._date,
                self._amount,
                self._units)

    def __str__(self):
        return '%s: %s%s' % (self._date, self._amount, self._units)


def read_file(file_name, units):
    """Read a CSV meter reading file into memory.

    Args:
        file_name (path str): the path to the file to read
        units (str): the units for the amounts read from the file

    Return:
        ([Measurement]): the data from the file
    """

    measurements = []
    with open(file_name, 'r') as f:
        reader = csv.reader(f)
        for row in reader:
            measurements += [Measurement(
                    datetime.datetime.strptime(row[0], '%Y-%m-%d').date(),
                    int(row[1]),
                    units)]
    return measurements


def get_plot_data(data):
    """Get the plottable data series for a given measurement dataset.

    Args:
        data ([Measurement]): the measurements to get plot data for

    Return:
        ([float], [float]): tuple of arrays of X data and Y data
    """

    x_data = []
    y_data = []
    for i in range(1, len(data)):
        num_days = (data[i].get_date() - data[i - 1].get_date()).days
        avg_usage = data[i].get_amount() / num_days
        x_data += [data[i].get_date()]
        y_data += [avg_usage]
    return (x_data, y_data)


def main(gas_file, elec_file):
    """Main program method.

    Args:
        gas_file (path str): the CSV for gas meter readings
        elec_file (path str): the CSV file for electric meter readings
    """

    gas_data = read_file(gas_file, 'therms')
    elec_data = read_file(elec_file, 'kWh')

    gas_plot_data = get_plot_data(gas_data)
    elec_plot_data = get_plot_data(elec_data)

    plt.plot(gas_plot_data[0], gas_plot_data[1], elec_plot_data[0], elec_plot_data[1])
    plt.show()


def parse_args():
    """Parse the command line arguments.

    Return:
        ({str: str}): the arguments
    """

    parser = argparse.ArgumentParser(description='Generate neat charts from ' +
            'utility usage data')
    parser.add_argument('gas', metavar='GAS_FILE', type=str,
            help='the CSV file that contains the gas meter readings')
    parser.add_argument('electric', metavar='ELEC_FILE', type=str,
            help='the CSV file that contains the electric meter readings')
    args = parser.parse_args()
    return {
        'gas': args.gas,
        'elec': args.electric
    }

if __name__ == '__main__':
    args = parse_args()
    main(args['gas'], args['elec'])
