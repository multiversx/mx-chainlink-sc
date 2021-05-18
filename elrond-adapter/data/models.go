package data

type JobRequest struct {
	JobID string      `json:"id"`
	Data  RequestData `json:"data"`
}

type JobResponse struct {
	JobRunID   string      `json:"jobRunID"`
	Data       interface{} `json:"data"`
	Result     interface{} `json:"result"`
	StatusCode int         `json:"statusCode"`
}

type RequestData struct {
	Value     string `json:"value"`
	Result    string `json:"result"`
	ScAddress string `json:"sc_address"`
	Function  string `json:"function"`
	RoundID   string `json:"round_id"`
}
