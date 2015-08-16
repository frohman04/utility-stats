import csv
import datetime
import dateutil
import logging
import os.path
import urllib.request


"""The URL to use to retrieve the temperature data.

str.format() Args:
    year (str): the year to get data for
    month (str): the month to get data for"""
TEMP_DATA_URL = ''.join([
        'http://www.wunderground.com/history/airport/KBED/{year}/{month}/1/',
        'MonthlyHistory.html?format=1'])
"""This module's logger."""
LOGGER = logging.getLogger(name=__name__)
"""The directory to save downloaded temperature data into."""
DATA_DIR = 'temp_data'


class TempDataManager(object):
    """Manager of temperature data retrieved from wunderground. Strives for
    efficiency by caching downloaded data on disk to minimize number of
    internet requests and by caching files in memory as they are requested to
    minimize amount of disk access needed.
    """

    def __init__(self):
        """Initialize the manager."""

        os.makedirs(DATA_DIR, exist_ok=True)
        self._cache = {}

    def get_temp(self, date):
        """Get the mean temperature in Farenheit for a given day.

        Args:
            date (datetime.date): the date to get the temperature for

        Return:
            int: the mean temperature in Farenheit
        """

        if not self._get_key(date) in self._cache:
            self._load_data(date)
        return self._cache[self._get_key(date)][date]

    def get_avg_temp(self, from_date, to_date):
        """Get the average temperature over a range of days, using each day's
        mean temperature in Farenheit as the data point to average.

        Args:
            from_date (datetime.date): the first date in the range (inclusive)
            to_date (datetime.date): the last date in the range (exclusive)

        Return:
            float: the average temperature in Farenheit
        """

        def daterange(start_date, end_date):
            """A generator that produces a range of dates.

            Args:
                from_date (datetime.date): the first date in the range
                        (inclusive)
                to_date (datetime.date): the last date in the range (exclusive)
            """

            for n in range(int((end_date - start_date).days)):
                yield start_date + datetime.timedelta(n)

        total = 0
        count = 0
        for date in daterange(from_date, to_date):
            total += self.get_temp(date)
            count += 1

        return total / count

    def _get_key(self, date):
        """Get the key into the cache for a given date.

        Args:
            date (datetime.date): the date to get the key for

        Return:
            str: the map key
        """

        return '%s-%s' % (date.year, date.month)

    def _load_data(self, date):
        """Load data from disk into cache for a given date.  May also cause
        data for other dates to be loaded at the same time.  If the data is
        not yet on disk, then download it.

        Args:
            date (datetime.date): the date who's data should be loded
        """

        # download the data if we don't have any data for the month
        data_file_name = os.path.join(
                DATA_DIR,
                '%s-%s.csv' % (date.year, date.month))
        if not os.path.exists(data_file_name):
            self._download_data(date.year, date.month, data_file_name)

        # load the data from file
        data = self._load_data_from_disk(data_file_name)

        # if the month doesn't have complete data, redownload it again
        last_day = max(sorted(data.keys()))
        first_day = datetime.date(last_day.year, last_day.month, 1)
        expected_last_day = (
                first_day +
                dateutil.relativedelta.relativedelta(months=+1) +
                datetime.timedelta(days=-1))
        if (last_day < expected_last_day and
                last_day != datetime.date.today()):
            self._download_data(date.year, date.month, data_file_name)
            data = self._load_data_from_disk(data_file_name)

        # save the downloaded data in memory
        self._cache[self._get_key(date)] = data

    def _load_data_from_disk(self, data_file_name):
        """Load the data contained in a downloaded file from disk.

        Args:
            data_file_name (path str): the file to load

        Return:
            {datetime.date: int}: the temperature in Farenheit for each day of
                    the month
        """

        data = {}
        with open(data_file_name, 'r') as f:
            reader = csv.DictReader(f)
            for row in reader:
                time_str = row['EST'] if 'EST' in row else row['EDT']
                date = datetime.datetime.strptime(time_str, '%Y-%m-%d').date()
                temp = int(row['Mean TemperatureF'])
                data[date] = temp
        return data

    def _download_data(self, year, month, out_file_name):
        """Download the temperature data for a given month.

        Args:
            year (int): the 4 digit year to download data for
            month (int): the numerical month (January = 1) to download data for
            out_file_name (path str): the file to save the data into
        """

        LOGGER.info('Downloading data for %s-%s' % (year, month))
        url = TEMP_DATA_URL.format(year=year, month=month)
        LOGGER.debug('\tURL: %s' % url)
        LOGGER.debug('\tOut File: %s' % out_file_name)
        with urllib.request.urlopen(url) as response, \
                open(out_file_name, 'wb') as out_file:
            data = response.read()
            text = data.decode('utf-8')
            text = text.replace('<br />', '')
            out_file.write(text[1:].encode('utf-8'))
