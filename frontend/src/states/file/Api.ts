import axios from "axios";

export const getDownloadBlob = async (jobId: string | undefined, path: string | undefined) => {
  const params = new URLSearchParams();
  params.append('path', path || "");
  const response = await axios.get(`/api/jobs/${jobId}/download?${params.toString()}`, {
    responseType: 'blob',
  });
  const contentDisposition = response.headers['content-disposition'];
  const fileName = contentDisposition?.split(';')[1]?.split('=')[1]?.replace(/"/g, '') || "download-file";

  const blobUrl = URL.createObjectURL(response.data);
  return { blobUrl, fileName };
};
export const getDownloadLink = async (jobId: string | undefined, path: string | undefined) => {
  const { blobUrl, fileName } = await getDownloadBlob(jobId, path);

  const link = document.createElement('a');
  link.href = blobUrl;
  link.download = fileName;
  return link;

}
export const download = async (jobId: string | undefined, path: string | undefined) => {
  const link = await getDownloadLink(jobId, path);
  document.body.appendChild(link);
  link.click();
  link.remove();
}
